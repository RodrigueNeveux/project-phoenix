use std::cell::Cell;

use gpui::Context;

use crate::app::PhoenixApp;
use crate::voice::stream::{Perks, Quality, SourceKind};
use crate::voice::{StreamSource, VoiceControl};

thread_local! {
    static QUALITY: Cell<Quality> = Cell::new(Quality::default());
}

pub fn quality() -> Quality {
    QUALITY.with(Cell::get)
}

pub fn set_quality(q: Quality) {
    QUALITY.with(|c| c.set(q));
}

pub fn perks(app: &PhoenixApp) -> Perks {
    let guild_tier = app
        .voice_channel
        .and_then(|c| app.channels.get(&c))
        .and_then(|c| c.guild)
        .or(app.selected_guild)
        .and_then(|g| app.guilds.iter().find(|x| x.id == g))
        .map_or(0, |g| g.boost_tier);
    Perks { nitro: app.backend.premium_tier2(), guild_tier }
}

pub fn live(app: &PhoenixApp) -> bool {
    app.voice.own_stream.is_some() || crate::voice::call::own_stream_pending()
}

pub fn start(app: &PhoenixApp, kind: SourceKind) {
    let q = quality();
    let (resolution, fps) = crate::voice::stream::resolve(q, perks(app));
    drop(app.backend.voice_control(VoiceControl::StartStream { source: StreamSource::Portal(kind), resolution, fps, sound: q.sound }));
}

pub fn stop(app: &PhoenixApp) {
    drop(app.backend.voice_control(VoiceControl::StopStream));
}

pub fn toggle(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    if live(app) {
        stop(app);
    } else {
        crate::ui::modals::go_live::open(None);
    }
    cx.notify();
}

pub fn label(app: &PhoenixApp) -> gpui::SharedString {
    if live(app) { crate::t!("voice.stop_share") } else { crate::t!("voice.share_screen") }
}
