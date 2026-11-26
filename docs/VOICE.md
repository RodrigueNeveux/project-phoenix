# Voice: how audio flows

`crates/phoenix-voice` holds the transport, `crates/phoenix-app/src/voice` drives it from the UI.

## Sending

microphone (cpal / PulseAudio) → `audio::device::encode_loop` (high-pass, noise suppression, gain, gate or push-to-talk, AGC) → Opus 48 kHz mono 64 kb/s with in-band FEC → `DaveSession::encrypt` → RTP → `TransportCipher` (`*_rtpsize`) → UDP. op 5 SPEAKING is sent when transmission starts and 5 silence frames close it.

## Receiving

UDP → `udp::classify_and_decrypt` → `rtp::parse_packet` → `connection::Receiver`:

1. the SSRC is mapped to a user from op 5 SPEAKING or op 12 VIDEO; packets of an unknown SSRC wait (25 per SSRC) until it is named;
2. `DaveSession::decrypt` removes the end-to-end layer with that sender's key ratchet (passthrough while DAVE is off, or 10 s after a transition);
3. the first non-silence frame raises `VoiceEvent::Speaking`, 300 ms without audio clears it;
4. the Opus frame goes to `audio::mixer::Mixer`: per-user jitter buffer, Opus decoder, FEC for a single loss, PLC for short bursts, `user volume × output volume`, deafen;
5. `audio::device::mix_loop` keeps ~60 ms queued on the output device chosen in the settings.

Go Live audio from a watched stream uses the same mixer, keyed `owner | STREAM_AUDIO_BIT`.

## Volumes

Sliders hold the value the client shows (perceptual, 0–200). The engine gets `voice::prefs::amplitude(value)`, the inverse of the client's module 792251: `n<1 ? n^2.8 : 10^((n-1)·6/20)` with `n = value/100`.

## Testing reception live

1. Someone else joins `〢🔊・phoenix-vocal` on Aperture Sciences and speaks (or plays music).
2. Start Phoenix on the account with `PHOENIX_SOUND_LOG=1` (add `PHOENIX_VOICE_TRACE=1` for gateway events), join the same channel, mic muted.
3. You should hear them, see their speaking ring, and stderr should print once per second:
   `voice-recv: user=<id> ssrc=<n> received=… decoded=… fec=… plc=… late=… level_db=-2x.x` and `voice-recv: mixer out_db=… queue_ms=60 underruns=0`.
   `decoded` must grow by ~50 per second while they speak; `level_db` above −60 means real audio.
4. Check the user menu: the volume slider and "Mute" change only that person; the headset button silences everything (`deafened=true`, `out_db=-100`).

Automated tests: `connection::receive_tests`, `audio::mixer::tests`, `rtp::jitter::tests`, `media::opus::tests`, `dave::session::tests`, `crypto::tests`.
