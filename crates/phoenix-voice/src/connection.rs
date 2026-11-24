use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::codec::Codec;
use crate::crypto::{EncryptionMode, TransportCipher};
use crate::dave::frame::OPUS_SILENCE;
use crate::dave::openmls_session::OpenMlsSession;
use crate::dave::session::{DaveAction, DaveSession};
use crate::gateway::protocol::speaking;
use crate::gateway::{self, GatewayEvent, GatewayHandle, ReconnectPolicy, ServerMessage, VoiceConnectInfo};
use crate::rtp::{self, RtpSender};
use crate::udp::{Incoming, MediaSocket};
use crate::{Error, Result};

const SPEAKING_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioOut {
    Frame(Vec<u8>),
    End,
}

#[derive(Debug, Clone)]
pub struct AudioIn {
    pub user_id: u64,
    pub ssrc: u32,
    pub sequence: u16,
    pub timestamp: u32,
    pub opus: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VoiceEvent {
    Connected { ssrc: u32, mode: String },
    Ping(u32),
    Speaking { user_id: u64, speaking: bool },
    Encryption { e2ee: bool, protocol_version: u16 },
    PrivacyCode(Option<String>),
    Stats(ConnStats),
    Reconnecting,
    Disconnected { reason: String },
    VideoState { user_id: u64, active: bool },
    Video { user_id: u64, frame: Vec<u8> },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ConnStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_lost: u64,
    pub outbound_loss_pct: Option<f32>,
    pub video_packets_sent: u64,
    pub video_rtcp_received: u64,
    pub video_frames_received: u64,
}

pub struct VoiceConfig {
    pub info: VoiceConnectInfo,
    pub channel_id: u64,
    pub user_id: u64,
}

#[derive(Debug)]
pub enum VideoOut {
    Start { width: u32, height: u32, fps: u32 },
    Frame(crate::media::EncodedVideo),
    Stop,
}

enum Cmd {
    Close,
    Video(VideoOut),
}

#[derive(Clone)]
pub struct VoiceConnection {
    tx: mpsc::UnboundedSender<Cmd>,
}

impl VoiceConnection {
    pub fn close(&self) {
        let _ = self.tx.send(Cmd::Close);
    }

    pub fn video(&self, v: VideoOut) {
        let _ = self.tx.send(Cmd::Video(v));
    }
}

pub fn connect(
    cfg: VoiceConfig,
    audio_out: mpsc::UnboundedReceiver<AudioOut>,
    audio_in: Box<dyn FnMut(AudioIn) + Send>,
) -> (VoiceConnection, mpsc::UnboundedReceiver<VoiceEvent>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (etx, erx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let reason = match run(cfg, rx, audio_out, audio_in, &etx).await {
            Ok(()) => "disconnected".to_string(),
            Err(e) => e.to_string(),
        };
        let _ = etx.send(VoiceEvent::Disconnected { reason });
    });
    (VoiceConnection { tx }, erx)
}

struct Media {
    socket: MediaSocket,
    ssrc: u32,
    sender: RtpSender,
    timestamp: u32,
    transmitting: bool,
    silence_left: u8,
    video_ssrc: u32,
    rtx_ssrc: u32,
    video: Option<VideoTx>,
}

struct VideoTx {
    sender: RtpSender,
    packetizer: crate::rtp::h264::H264Packetizer,
    base: u32,
}

pub fn video_payload(audio_ssrc: u32, video_ssrc: u32, rtx_ssrc: u32, active: Option<(u32, u32, u32)>) -> gateway::protocol::VideoPayload {
    stream_payload("video", audio_ssrc, video_ssrc, rtx_ssrc, active)
}

pub fn stream_payload(
    kind: &str,
    audio_ssrc: u32,
    video_ssrc: u32,
    rtx_ssrc: u32,
    active: Option<(u32, u32, u32)>,
) -> gateway::protocol::VideoPayload {
    use gateway::protocol::{Resolution, StreamDescriptor, VideoPayload};
    let stream = StreamDescriptor {
        kind: kind.into(),
        rid: "100".into(),
        ssrc: video_ssrc,
        rtx_ssrc,
        active: active.is_some(),
        quality: 100,
        max_bitrate: Some(2_500_000),
        max_framerate: Some(active.map_or(20, |(_, _, fps)| fps)),
        max_resolution: Some(active.map_or(Resolution { kind: "fixed".into(), width: 1280, height: 720 }, |(width, height, _)| Resolution {
            kind: "fixed".into(),
            width,
            height,
        })),
    };
    VideoPayload {
        user_id: None,
        audio_ssrc,
        video_ssrc: if active.is_some() { video_ssrc } else { 0 },
        rtx_ssrc: if active.is_some() { rtx_ssrc } else { 0 },
        streams: vec![stream],
    }
}

fn send_dave_actions(gw: &GatewayHandle, actions: Vec<DaveAction>) {
    for a in actions {
        let r = match a {
            DaveAction::KeyPackage(kp) => gw.dave_key_package(kp),
            DaveAction::CommitWelcome(cw) => gw.dave_commit_welcome(&cw),
            DaveAction::TransitionReady(t) => gw.dave_transition_ready(t),
            DaveAction::InvalidCommitWelcome(t) => gw.dave_invalid_commit_welcome(t),
        };
        if let Err(e) = r {
            tracing::warn!(error = %e, "dave: send failed");
        }
    }
}

async fn run(
    cfg: VoiceConfig,
    mut cmds: mpsc::UnboundedReceiver<Cmd>,
    mut audio_out: mpsc::UnboundedReceiver<AudioOut>,
    mut audio_in: Box<dyn FnMut(AudioIn) + Send>,
    events: &mpsc::UnboundedSender<VoiceEvent>,
) -> Result<()> {
    let (gw, mut gev, _task) = gateway::spawn(cfg.info.clone(), None, ReconnectPolicy::default());
    let mls = OpenMlsSession::new(cfg.user_id, cfg.channel_id)?;
    let mut dave = DaveSession::new(Box::new(mls), cfg.user_id);
    let mut media: Option<Media> = None;
    let mut ssrc_users: HashMap<u32, u64> = HashMap::new();
    let mut last_heard: HashMap<u64, Instant> = HashMap::new();
    let mut was_e2ee = false;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    let mut recv_buf_errors = 0u32;
    let mut audio_open = true;
    let mut stats = ConnStats::default();
    let mut last_seq: HashMap<u32, u16> = HashMap::new();
    let mut last_code: Option<String> = None;
    let mut ticks = 0u32;
    let mut video_users: HashMap<u32, u64> = HashMap::new();
    let mut depack: HashMap<u32, crate::rtp::h264::H264Depacketizer> = HashMap::new();

    loop {
        tokio::select! {
            ev = gev.recv() => {
                let Some(ev) = ev else { return Err(Error::Closed { code: None, reason: "gateway task ended".into() }) };
                match ev {
                    GatewayEvent::Message(m) => match m {
                        ServerMessage::Ready(r) => {
                            let addr = (r.ip.as_str(), r.port)
                                .to_socket_addrs()?
                                .next()
                                .ok_or(Error::Protocol("voice server address".into()))?;
                            let socket = MediaSocket::connect(addr).await?;
                            let (ip, port) = socket.discover(r.ssrc).await?;
                            let mode = EncryptionMode::choose(&r.modes).ok_or(Error::Unsupported("no common encryption mode".into()))?;
                            let caps = crate::media::video::probe_capabilities();
                            gw.select_protocol(&ip, port, mode.name(), &caps.offer())?;
                            let want = if cfg.info.stream { "screen" } else { "video" };
                            let vs = r.streams.iter().find(|s| s.kind == want).map_or((0, 0), |s| (s.ssrc, s.rtx_ssrc));
                            media = Some(Media {
                                socket,
                                ssrc: r.ssrc,
                                sender: RtpSender::new(r.ssrc, Codec::Opus.payload_type()),
                                timestamp: rand::random(),
                                transmitting: false,
                                silence_left: 0,
                                video_ssrc: vs.0,
                                rtx_ssrc: vs.1,
                                video: None,
                            });
                        }
                        ServerMessage::SessionDescription(sd) => {
                            let m = media.as_mut().ok_or(Error::Protocol("session description before ready".into()))?;
                            let mode = EncryptionMode::from_name(&sd.mode).ok_or(Error::Unsupported(sd.mode.clone()))?;
                            m.socket.set_cipher(TransportCipher::new(mode, &sd.secret_key)?);
                            send_dave_actions(&gw, dave.on_session_description(sd.dave_protocol_version));
                            let _ = events.send(VoiceEvent::Connected { ssrc: m.ssrc, mode: sd.mode.clone() });
                        }
                        ServerMessage::HeartbeatAck { nonce } => {
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                            if nonce > 0 && nonce <= now && now - nonce < 60_000 {
                                let _ = events.send(VoiceEvent::Ping((now - nonce) as u32));
                            }
                        }
                        ServerMessage::Speaking { user_id, ssrc, .. } => {
                            if let Some(uid) = user_id.and_then(|u| u.parse::<u64>().ok()) {
                                ssrc_users.insert(ssrc, uid);
                                dave.add_users([uid]);
                            }
                        }
                        ServerMessage::Video(v) => {
                            if let Some(uid) = v.user_id.as_deref().and_then(|u| u.parse::<u64>().ok()) {
                                if v.audio_ssrc != 0 {
                                    ssrc_users.insert(v.audio_ssrc, uid);
                                    dave.add_users([uid]);
                                }
                                video_users.retain(|_, u| *u != uid);
                                if v.video_ssrc != 0 {
                                    video_users.insert(v.video_ssrc, uid);
                                }
                                let _ = events.send(VoiceEvent::VideoState { user_id: uid, active: v.video_ssrc != 0 });
                            }
                        }
                        ServerMessage::ClientsConnect { user_ids } => {
                            dave.add_users(user_ids.iter().filter_map(|u| u.parse().ok()));
                        }
                        ServerMessage::ClientDisconnect { user_id } => {
                            if let Ok(uid) = user_id.parse::<u64>() {
                                dave.remove_user(uid);
                                ssrc_users.retain(|_, u| *u != uid);
                                if last_heard.remove(&uid).is_some() {
                                    let _ = events.send(VoiceEvent::Speaking { user_id: uid, speaking: false });
                                }
                            }
                        }
                        ServerMessage::Dave(d) => {
                            let actions = dave.handle(&d);
                            send_dave_actions(&gw, actions);
                        }
                        _ => {}
                    },
                    GatewayEvent::Reconnecting { .. } => {
                        let _ = events.send(VoiceEvent::Reconnecting);
                    }
                    GatewayEvent::Closed { code, reason } => {
                        return if code == Some(1000) { Ok(()) } else { Err(Error::Closed { code, reason }) };
                    }
                }
                let e2ee = dave.is_active();
                if e2ee != was_e2ee {
                    was_e2ee = e2ee;
                    let _ = events.send(VoiceEvent::Encryption { e2ee, protocol_version: dave.protocol_version() });
                }
                let code = if e2ee { dave.epoch_authenticator().and_then(|a| crate::dave::privacy_code(&a)) } else { None };
                if code != last_code {
                    last_code = code.clone();
                    let _ = events.send(VoiceEvent::PrivacyCode(code));
                }
            }
            pkt = async {
                match media.as_ref() {
                    Some(m) => m.socket.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                let pkt = match pkt {
                    Ok(Incoming::Rtp(p)) => p,
                    Ok(Incoming::Rtcp(r)) => {
                        if let Some(m) = media.as_ref()
                            && m.video.is_some()
                            && r.windows(4).any(|w| w == m.video_ssrc.to_be_bytes())
                        {
                            stats.video_rtcp_received += 1;
                        }
                        if let Some(m) = media.as_ref()
                            && let Some((_, frac)) = rtp::rtcp::report_blocks(&r).into_iter().find(|(ssrc, _)| *ssrc == m.ssrc)
                        {
                            stats.outbound_loss_pct = Some(frac as f32 * 100.0 / 256.0);
                        }
                        continue;
                    }
                    Err(Error::Io(e)) => return Err(Error::Io(e)),
                    Err(e) => {
                        recv_buf_errors += 1;
                        if recv_buf_errors.is_power_of_two() {
                            tracing::debug!(error = %e, "voice: dropped packet");
                        }
                        continue;
                    }
                };
                let Ok((h, payload)) = rtp::parse_packet(&pkt) else { continue };
                if h.payload_type == Codec::H264.payload_type() {
                    let Some(&uid) = video_users.get(&h.ssrc) else { continue };
                    let d = depack.entry(h.ssrc).or_default();
                    let au = match rtp::Depacketizer::push(d, payload, h.marker) {
                        Ok(Some(au)) => au,
                        Ok(None) => continue,
                        Err(_) => {
                            *d = Default::default();
                            continue;
                        }
                    };
                    stats.video_frames_received += 1;
                    if let Ok(frame) = dave.decrypt(uid, &au) {
                        let _ = events.send(VoiceEvent::Video { user_id: uid, frame });
                    }
                    continue;
                }
                if h.payload_type != Codec::Opus.payload_type() {
                    continue;
                }
                stats.packets_received += 1;
                if let Some(prev) = last_seq.insert(h.ssrc, h.sequence) {
                    let d = h.sequence.wrapping_sub(prev);
                    if (2..1000).contains(&d) {
                        stats.packets_lost += u64::from(d - 1);
                    } else if d == 0 || d >= 32768 {
                        last_seq.insert(h.ssrc, prev);
                    }
                }
                let Some(&uid) = ssrc_users.get(&h.ssrc) else { continue };
                let opus = match dave.decrypt(uid, payload) {
                    Ok(f) => f,
                    Err(e) => {
                        recv_buf_errors += 1;
                        if recv_buf_errors.is_power_of_two() {
                            tracing::debug!(error = %e, user = uid, "voice: frame decrypt failed");
                        }
                        continue;
                    }
                };
                if opus != OPUS_SILENCE && last_heard.insert(uid, Instant::now()).is_none() {
                    let _ = events.send(VoiceEvent::Speaking { user_id: uid, speaking: true });
                }
                audio_in(AudioIn { user_id: uid, ssrc: h.ssrc, sequence: h.sequence, timestamp: h.timestamp, opus });
            }
            frame = audio_out.recv(), if audio_open => {
                let Some(frame) = frame else { audio_open = false; continue };
                let Some(m) = media.as_mut() else { continue };
                let opus = match frame {
                    AudioOut::Frame(f) => {
                        if !m.transmitting {
                            m.transmitting = true;
                            gw.speaking(speaking::MICROPHONE, m.ssrc)?;
                            let _ = events.send(VoiceEvent::Speaking { user_id: cfg.user_id, speaking: true });
                        }
                        m.silence_left = 5;
                        f
                    }
                    AudioOut::End => {
                        if m.transmitting {
                            m.transmitting = false;
                            let _ = events.send(VoiceEvent::Speaking { user_id: cfg.user_id, speaking: false });
                        }
                        if m.silence_left == 0 {
                            continue;
                        }
                        m.silence_left -= 1;
                        if m.silence_left == 0 {
                            gw.speaking(0, m.ssrc)?;
                        }
                        OPUS_SILENCE.to_vec()
                    }
                };
                let Ok(payload) = dave.encrypt(Codec::Opus, &opus) else { continue };
                m.timestamp = m.timestamp.wrapping_add(crate::FRAME_SAMPLES as u32);
                for p in m.sender.wrap(vec![payload], m.timestamp) {
                    match m.socket.send_rtp(&p).await {
                        Ok(()) => stats.packets_sent += 1,
                        Err(e) => tracing::debug!(error = %e, "voice: send failed"),
                    }
                }
            }
            _ = tick.tick() => {
                let now = Instant::now();
                last_heard.retain(|uid, t| {
                    let alive = now.duration_since(*t) < SPEAKING_TIMEOUT;
                    if !alive {
                        let _ = events.send(VoiceEvent::Speaking { user_id: *uid, speaking: false });
                    }
                    alive
                });
                ticks += 1;
                if ticks.is_multiple_of(10) && media.is_some() {
                    let _ = events.send(VoiceEvent::Stats(stats));
                }
            }
            cmd = cmds.recv() => match cmd {
                Some(Cmd::Close) | None => {
                    let _ = gw.close();
                    return Ok(());
                }
                Some(Cmd::Video(v)) => {
                    let Some(m) = media.as_mut() else { continue };
                    match v {
                        VideoOut::Start { width, height, fps } => {
                            if m.video_ssrc == 0 {
                                tracing::warn!("voice: no video ssrc from ready");
                                continue;
                            }
                            gw.video(&stream_payload(if cfg.info.stream { "screen" } else { "video" }, m.ssrc, m.video_ssrc, m.rtx_ssrc, Some((width, height, fps))))?;
                            m.video = Some(VideoTx {
                                sender: RtpSender::new(m.video_ssrc, Codec::H264.payload_type()),
                                packetizer: crate::rtp::h264::H264Packetizer::new(),
                                base: rand::random(),
                            });
                        }
                        VideoOut::Stop => {
                            if m.video.take().is_some() {
                                gw.video(&stream_payload(if cfg.info.stream { "screen" } else { "video" }, m.ssrc, m.video_ssrc, m.rtx_ssrc, None))?;
                            }
                        }
                        VideoOut::Frame(e) => {
                            let Some(vt) = m.video.as_mut() else { continue };
                            let au = vt.packetizer.prepare_access_unit(&e.data);
                            let Ok(au) = dave.encrypt(Codec::H264, &au) else { continue };
                            let Ok(payloads) = vt.packetizer.packetize_prepared(&au, rtp::DEFAULT_MAX_PAYLOAD) else { continue };
                            let ts = vt.base.wrapping_add((e.pts_us.wrapping_mul(90) / 1000) as u32);
                            for p in vt.sender.wrap(payloads, ts) {
                                match m.socket.send_rtp(&p).await {
                                    Ok(()) => stats.video_packets_sent += 1,
                                    Err(e) => tracing::debug!(error = %e, "voice: video send failed"),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod video_tests {
    use super::*;

    #[test]
    fn camera_op12_active_and_stopped() {
        let on = video_payload(1, 2, 3, Some((1280, 720, 30)));
        assert_eq!((on.video_ssrc, on.rtx_ssrc), (2, 3));
        let s = &on.streams[0];
        assert!(s.active);
        assert_eq!(s.max_framerate, Some(30));
        assert_eq!(s.max_resolution.as_ref().map(|r| (r.width, r.height)), Some((1280, 720)));
        let off = video_payload(1, 2, 3, None);
        assert_eq!(off.video_ssrc, 0);
        assert!(!off.streams[0].active);
        assert_eq!(off.streams[0].ssrc, 2);
    }
}
