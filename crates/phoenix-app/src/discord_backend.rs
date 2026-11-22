//! Real [`Backend`] on top of `phoenix-api`.
//!
//! Networking runs on a private multi-threaded tokio runtime (its own worker
//! threads), never on GPUI's executor: every trait method spawns onto that
//! runtime and hands the UI a `JoinHandle` to await. Gateway dispatches are
//! applied to a [`Cache`] and translated into [`AppEvent`]s by
//! [`convert::Adapter`].

pub mod convert;
pub mod http;
pub mod store;

use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use phoenix_api::{
    Cache, ClientProperties, Error as ApiError, Gateway, GatewayConfig, GatewayEvent, GatewayHandle, RestClient,
    gateway::GuildSubscription,
    models as api,
    rest::{CreateMessage, LoginResponse, MessageQuery},
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    backend::{AppEvent, Backend, BackendError, BoxFuture, Credentials, EventSink, ReadyData},
    model::{Id, Message},
};
use convert::{Adapter, Prefs, sf};
use store::{KeyringStore, Secret, TokenStore};

const READY_TIMEOUT: Duration = Duration::from_secs(45);
/// Delay before acking a channel that keeps receiving messages while open.
const TYPING_INTERVAL: Duration = Duration::from_secs(8);
const ACK_DEBOUNCE: Duration = Duration::from_millis(1500);

fn err(s: impl Into<String>) -> BackendError {
    BackendError(s.into())
}

/// Human-readable login/API error. Never includes the token (the API error
/// types don't carry it).
fn api_err(e: ApiError) -> BackendError {
    match &e {
        ApiError::CaptchaRequired { .. } => {
            err("Discord asked for a captcha, which Phoenix can't show yet. Log in with a token instead.")
        }
        ApiError::Api { body, .. } if e.is_unauthorized() || body.code == 50035 => {
            err(if body.message.is_empty() { "Login or password is invalid.".to_owned() } else { body.message.clone() })
        }
        _ if e.is_unauthorized() => err("Invalid token."),
        ApiError::RateLimited { retry_after, .. } => {
            err(format!("Rate limited by Discord, retry in {}s.", retry_after.as_secs().max(1)))
        }
        _ => err(e.to_string()),
    }
}

struct Session {
    rest: RestClient,
    gateway: GatewayHandle,
    cache: Cache,
    adapter: Arc<Mutex<Adapter>>,
    selected_channel: Mutex<Option<api::Snowflake>>,
    /// Channels with a debounced ack in flight.
    pending_acks: Mutex<HashSet<u64>>,
    /// Guilds already subscribed via op 37 (per channel member list).
    subscribed: Mutex<HashSet<(u64, u64)>>,
    /// Last typing notification (channel, when): Discord shows it for ~10s.
    last_typing: Mutex<Option<(u64, Instant)>>,
}

struct Inner {
    rt: tokio::runtime::Runtime,
    sink: Mutex<Option<EventSink>>,
    session: Mutex<Option<Arc<Session>>>,
    mfa_ticket: Mutex<Option<Secret>>,
    store: Arc<dyn TokenStore>,
    props: ClientProperties,
}

impl Inner {
    fn emit(&self, ev: AppEvent) {
        if let Some(s) = self.sink.lock().as_ref() {
            s(ev);
        }
    }
    fn session(&self) -> Result<Arc<Session>, BackendError> {
        self.session.lock().clone().ok_or_else(|| err("Not logged in."))
    }
}

#[derive(Clone)]
pub struct DiscordBackend {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for DiscordBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordBackend").field("logged_in", &self.inner.session.lock().is_some()).finish_non_exhaustive()
    }
}

impl DiscordBackend {
    /// Backend storing the token in the OS keyring.
    pub fn new() -> Self {
        Self::with_store(Arc::new(KeyringStore))
    }

    pub fn with_store(store: Arc<dyn TokenStore>) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("phoenix-net")
            .enable_all()
            .build()
            .expect("tokio runtime");
        Self {
            inner: Arc::new(Inner {
                rt,
                sink: Mutex::new(None),
                session: Mutex::new(None),
                mfa_ticket: Mutex::new(None),
                store,
                props: ClientProperties::linux_desktop(),
            }),
        }
    }

    /// HTTP client for GPUI image loading (`cx.set_http_client`).
    pub fn http_client(&self) -> Arc<dyn gpui::http_client::HttpClient> {
        Arc::new(http::CdnHttpClient::new(self.inner.rt.handle().clone(), self.inner.props.user_agent()))
    }

    fn spawn<T, F>(&self, f: impl FnOnce(Arc<Inner>) -> F) -> BoxFuture<T>
    where
        T: Send + 'static,
        F: std::future::Future<Output = Result<T, BackendError>> + Send + 'static,
    {
        let handle = self.inner.rt.spawn(f(self.inner.clone()));
        Box::pin(async move { handle.await.map_err(|e| err(format!("internal error: {e}")))? })
    }

    fn with_session<T, F>(&self, f: impl FnOnce(Arc<Inner>, Arc<Session>) -> F + Send + 'static) -> BoxFuture<T>
    where
        T: Send + 'static,
        F: std::future::Future<Output = Result<T, BackendError>> + Send + 'static,
    {
        self.spawn(move |inner| async move {
            let s = inner.session()?;
            f(inner, s).await
        })
    }
}

impl Default for DiscordBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Tolerate tokens pasted with quotes or whitespace.
fn clean_token(t: &str) -> String {
    t.trim().trim_matches('"').trim().to_owned()
}

async fn store_op<T: Send + 'static>(
    store: &Arc<dyn TokenStore>,
    f: impl FnOnce(&dyn TokenStore) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let s = store.clone();
    tokio::task::spawn_blocking(move || f(&*s)).await.map_err(|e| e.to_string())?
}

/// Connect the gateway with `token`, wait for READY, start the event pump.
async fn connect(inner: Arc<Inner>, token: Secret, remember: bool) -> Result<ReadyData, BackendError> {
    if let Some(old) = inner.session.lock().take() {
        old.gateway.shutdown();
    }
    let rest = RestClient::new(token.expose(), inner.props.clone());
    // Cheap validation first: a bad token fails here with a clear 401.
    rest.get_me().await.map_err(api_err)?;

    let mut cfg = GatewayConfig::new(token.expose());
    cfg.properties = inner.props.clone();
    let (gateway, mut rx) = Gateway::spawn(cfg);
    let cache = Cache::default();

    let ready = tokio::time::timeout(READY_TIMEOUT, async {
        while let Some(ev) = rx.recv().await {
            cache.apply(&ev);
            match ev {
                GatewayEvent::Ready(r) => return Ok(r),
                GatewayEvent::Fatal { code, reason } => return Err(err(convert::fatal_message(code, &reason))),
                _ => {}
            }
        }
        Err(err("Gateway closed before READY."))
    })
    .await
    .map_err(|_| err("Timed out waiting for Discord."))
    .flatten();
    let ready = match ready {
        Ok(r) => r,
        Err(e) => {
            gateway.shutdown();
            return Err(e);
        }
    };

    if remember && let Err(e) = store_op(&inner.store, move |s| s.save(&token)).await {
        tracing_warn(&format!("could not save token to the keyring: {e}"));
    }

    let prefs = Prefs::from_ready(&ready);
    let data = convert::build_ready(&cache, &ready, &prefs);
    let adapter = Arc::new(Mutex::new(Adapter::new(ready.user.id, prefs)));
    let session = Arc::new(Session {
        rest,
        gateway,
        cache,
        adapter,
        selected_channel: Mutex::new(None),
        pending_acks: Mutex::new(HashSet::new()),
        subscribed: Mutex::new(HashSet::new()),
        last_typing: Mutex::new(None),
    });
    *inner.session.lock() = Some(session.clone());
    tokio::spawn(pump(inner.clone(), session, rx));
    Ok(data)
}

fn tracing_warn(msg: &str) {
    eprintln!("discord-phoenix: {msg}");
}

/// Gateway events -> cache -> AppEvents, until the session ends.
async fn pump(inner: Arc<Inner>, s: Arc<Session>, mut rx: UnboundedReceiver<GatewayEvent>) {
    while let Some(ev) = rx.recv().await {
        // Logged out / replaced: stop delivering.
        if !inner.session.lock().as_ref().is_some_and(|cur| Arc::ptr_eq(cur, &s)) {
            return;
        }
        s.cache.apply(&ev);
        let events = s.adapter.lock().map(&s.cache, &ev);
        if let GatewayEvent::MessageCreate(m) = &ev
            && *s.selected_channel.lock() == Some(m.channel_id)
            && m.author.id != s.adapter.lock().me
        {
            schedule_ack(&s, m.channel_id, m.id);
        }
        let fatal = matches!(ev, GatewayEvent::Fatal { .. });
        for e in events {
            inner.emit(e);
        }
        if let GatewayEvent::Fatal { code, .. } = ev {
            if code == 4004 {
                let _ = store_op(&inner.store, |st| st.clear()).await;
            }
        }
        if fatal {
            inner.session.lock().take();
            return;
        }
    }
}

fn schedule_ack(s: &Arc<Session>, channel: api::Snowflake, msg: api::Snowflake) {
    s.adapter.lock().mark_read(channel.0, msg.0);
    if !s.pending_acks.lock().insert(channel.0) {
        return;
    }
    let s = s.clone();
    tokio::spawn(async move {
        tokio::time::sleep(ACK_DEBOUNCE).await;
        s.pending_acks.lock().remove(&channel.0);
        let last = s.cache.channel(channel).and_then(|c| c.last_message_id).unwrap_or(msg);
        let _ = s.rest.ack_message(channel, last).await;
    });
}

impl Backend for DiscordBackend {
    fn set_event_sink(&self, sink: EventSink) {
        *self.inner.sink.lock() = Some(sink);
    }

    fn login(&self, creds: Credentials) -> BoxFuture<ReadyData> {
        self.spawn(move |inner| async move {
            match creds {
                Credentials::Token(t) => {
                    let t = clean_token(&t);
                    if t.is_empty() {
                        return Err(err("Enter a token."));
                    }
                    connect(inner, Secret::new(t), true).await
                }
                Credentials::Password { login, password } => {
                    if login.trim().is_empty() || password.is_empty() {
                        return Err(err("Enter your email and password."));
                    }
                    let rest = RestClient::unauthenticated(inner.props.clone());
                    match rest.login(login.trim(), &password).await.map_err(api_err)? {
                        LoginResponse::Token { token, .. } => connect(inner, Secret::new(token), true).await,
                        LoginResponse::MfaRequired { ticket, totp, .. } => {
                            if !totp {
                                return Err(err(
                                    "This account uses SMS or a security key for 2FA, which isn't supported yet. Log in with a token instead.",
                                ));
                            }
                            *inner.mfa_ticket.lock() = Some(Secret::new(ticket));
                            Err(err("Enter the 6-digit code from your authenticator app."))
                        }
                    }
                }
                Credentials::Mfa { code } => {
                    let ticket = inner.mfa_ticket.lock().clone().ok_or_else(|| err("Log in with your password first."))?;
                    let code: String = code.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
                    let rest = RestClient::unauthenticated(inner.props.clone());
                    let token = rest.login_mfa_totp(ticket.expose(), &code).await.map_err(api_err)?;
                    inner.mfa_ticket.lock().take();
                    connect(inner, Secret::new(token), true).await
                }
            }
        })
    }

    fn restore_session(&self) -> Option<BoxFuture<ReadyData>> {
        Some(self.spawn(move |inner| async move {
            let tok = store_op(&inner.store, |s| s.load()).await.map_err(|e| {
                tracing_warn(&format!("keyring unavailable: {e}"));
                err("")
            })?;
            let Some(tok) = tok else { return Err(err("")) };
            let res = connect(inner.clone(), tok, false).await;
            if let Err(e) = &res
                && e.0 == "Invalid token."
            {
                let _ = store_op(&inner.store, |s| s.clear()).await;
            }
            res
        }))
    }

    fn needs_mfa(&self) -> bool {
        self.inner.mfa_ticket.lock().is_some()
    }

    fn logout(&self) -> BoxFuture<()> {
        self.spawn(move |inner| async move {
            inner.mfa_ticket.lock().take();
            if let Some(s) = inner.session.lock().take() {
                s.gateway.shutdown();
            }
            store_op(&inner.store, |s| s.clear()).await.map_err(err)
        })
    }

    fn select_guild(&self, guild: Option<Id>) -> BoxFuture<()> {
        self.with_session(move |_, s| async move {
            if let Some(g) = guild {
                let _ = s.gateway.subscribe_guild(sf(g), GuildSubscription::focused(None));
            }
            Ok(())
        })
    }

    fn select_channel(&self, channel: Id) -> BoxFuture<()> {
        self.with_session(move |_, s| async move {
            let ch = sf(channel);
            *s.selected_channel.lock() = Some(ch);
            let c = s.cache.channel(ch);
            // Op 37 with a channel range populates the member list + presences.
            if let Some(g) = c.as_ref().and_then(|c| c.guild_id)
                && s.subscribed.lock().insert((g.0, ch.0))
            {
                let _ = s.gateway.subscribe_guild(g, GuildSubscription::focused(Some(ch)));
            }
            if let Some(last) = c.and_then(|c| c.last_message_id) {
                let was_unread = {
                    let mut a = s.adapter.lock();
                    let unread = a.prefs.reads.get(&ch.0).is_none_or(|r| r.last_read < last.0 || r.mentions > 0);
                    a.mark_read(ch.0, last.0);
                    unread
                };
                if was_unread {
                    s.rest.ack_message(ch, last).await.map_err(api_err)?;
                }
            }
            Ok(())
        })
    }

    fn load_messages(&self, channel: Id, before: Option<Id>, limit: u32) -> BoxFuture<Vec<Message>> {
        self.with_session(move |inner, s| async move {
            let limit = limit.clamp(1, 100) as u8;
            let q = match before {
                Some(b) => MessageQuery::before(sf(b), limit),
                None => MessageQuery::latest(limit),
            };
            let mut msgs = s.rest.get_messages(sf(channel), q).await.map_err(api_err)?;
            msgs.sort_by_key(|m| m.id);
            let mut users = Vec::new();
            for m in &msgs {
                convert::message_users(m, &mut users);
            }
            // Delivered before the page so author names resolve on first render.
            inner.emit(AppEvent::Users(users));
            let out = msgs.iter().map(convert::message).collect();
            s.cache.insert_messages(sf(channel), msgs);
            Ok(out)
        })
    }

    fn send_message(&self, channel: Id, content: String, reply_to: Option<Id>) -> BoxFuture<Message> {
        self.with_session(move |_, s| async move {
            let mut msg = CreateMessage::text(content);
            if let Some(r) = reply_to {
                msg = msg.reply_to(sf(channel), sf(r), true);
            }
            let m = s.rest.send_message(sf(channel), msg).await.map_err(api_err)?;
            s.adapter.lock().mark_read(m.channel_id.0, m.id.0);
            Ok(convert::message(&m))
        })
    }

    fn edit_message(&self, channel: Id, msg: Id, content: String) -> BoxFuture<Message> {
        self.with_session(move |_, s| async move {
            let m = s.rest.edit_message(sf(channel), sf(msg), &content).await.map_err(api_err)?;
            Ok(convert::message(&m))
        })
    }

    fn delete_message(&self, channel: Id, msg: Id) -> BoxFuture<()> {
        self.with_session(move |_, s| async move { s.rest.delete_message(sf(channel), sf(msg)).await.map_err(api_err) })
    }

    fn trigger_typing(&self, channel: Id) -> BoxFuture<()> {
        self.with_session(move |_, s| async move {
            {
                let mut last = s.last_typing.lock();
                if last.is_some_and(|(c, t)| c == channel.0 && t.elapsed() < TYPING_INTERVAL) {
                    return Ok(());
                }
                *last = Some((channel.0, Instant::now()));
            }
            s.rest.trigger_typing(sf(channel)).await.map_err(api_err)
        })
    }

    fn join_voice(&self, _channel: Option<Id>) -> BoxFuture<()> {
        // phoenix-voice isn't wired to the UI yet; announcing a voice state
        // without a media connection would show us as connected-but-silent.
        Box::pin(async { Err(err("Voice isn't available in this build yet.")) })
    }
}
