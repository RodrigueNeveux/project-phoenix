//! "Voice Connected" panel above the user panel, and the speaking ring used
//! on voice channel member avatars.

use gpui::{AnyElement, Context, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px, svg};

use crate::app::PhoenixApp;
use crate::assets::Icon;
use crate::theme::{Palette, c};
use crate::ui::icon_button;
use crate::voice::{Quality, VoiceStatus};

const CAMERA: &str = "icons/voice/camera.svg";
const SCREEN: &str = "icons/voice/screen.svg";
const SOUNDBOARD: &str = "icons/voice/soundboard.svg";
const ACTIVITY: &str = "icons/voice/activity.svg";
const SHIELD: &str = "icons/voice/shield.svg";

/// Icons owned by this module (hooked into the asset source).
pub fn svg_asset(path: &str) -> Option<&'static str> {
    Some(match path {
        CAMERA => concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black">"#,
            r#"<path d="M4 5h11a2 2 0 0 1 2 2v2.4l3.6-2.6A1 1 0 0 1 22 7.6v8.8a1 1 0 0 1-1.4.8L17 14.6V17a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z"/>"#,
            "</svg>"
        ),
        SCREEN => concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black">"#,
            r#"<path fill-rule="evenodd" d="M4 3h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-6v2h3a1 1 0 1 1 0 2H7a1 1 0 1 1 0-2h3v-2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zm8 3-4 4h3v3h2v-3h3z"/>"#,
            "</svg>"
        ),
        SOUNDBOARD => concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black">"#,
            r#"<path d="M12 3a9 9 0 0 1 9 9 1 1 0 1 1-2 0 7 7 0 0 0-14 0v2h2a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 9-9zm3 9h2a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2z"/>"#,
            "</svg>"
        ),
        ACTIVITY => concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black">"#,
            r#"<path fill-rule="evenodd" d="M7 6h10a5 5 0 0 1 5 5v2a5 5 0 0 1-8.5 3.5h-3A5 5 0 0 1 2 13v-2a5 5 0 0 1 5-5zm0 3v1.5H5.5v2H7V14h2v-1.5h1.5v-2H9V9zm9.5 0a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4zm-2 2.6a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4z"/>"#,
            "</svg>"
        ),
        SHIELD => concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black">"#,
            r#"<path d="M12 2 4 5v6c0 5 3.4 9.4 8 11 4.6-1.6 8-6 8-11V5z"/>"#,
            "</svg>"
        ),
        _ => return None,
    })
}

fn asset_icon(path: &'static str, size: f32, color: Hsla) -> gpui::Svg {
    svg().path(SharedString::new_static(path)).size(px(size)).flex_none().text_color(color)
}

/// Three rising bars, Discord's connection-quality glyph.
fn signal_bars(color: Hsla) -> impl IntoElement {
    div()
        .flex()
        .items_end()
        .gap(px(2.))
        .h(px(14.))
        .flex_none()
        .children([6., 10., 14.].map(|h| div().w(px(3.)).h(px(h)).rounded(px(1.)).bg(color)))
}

/// Wrap an avatar in the green speaking ring (Discord: 2px ring with a gap).
pub fn speaking_ring(avatar: AnyElement, size: f32, speaking: bool) -> AnyElement {
    div()
        .size(px(size + 4.))
        .flex_none()
        .rounded_full()
        .border_2()
        .border_color(if speaking { c(Palette::GREEN) } else { gpui::transparent_black() })
        .flex()
        .items_center()
        .justify_center()
        .child(avatar)
        .into_any_element()
}

fn action_button(id: &'static str, path: &'static str, label: &'static str, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .h(px(32.))
        .rounded(px(8.))
        .bg(c(Palette::BG_MOD_HOVER))
        .hover(|s| s.bg(c(0x404249)))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .child(asset_icon(path, 20., c(Palette::INTERACTIVE)))
        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(label).build(window, cx))
        .on_click(cx.listener(move |a, _, _, cx| a.stub(label, cx)))
}

pub fn voice_panel(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let ch = app.voice_channel.and_then(|v| app.channels.get(&v));
    let guild = ch
        .and_then(|c| c.guild)
        .and_then(|g| app.guilds.iter().find(|x| x.id == g))
        .map(|g| g.name.clone())
        .unwrap_or_default();
    let name = ch.map(|c| c.name.to_string()).unwrap_or_default();
    let label: SharedString = if guild.is_empty() { name.into() } else { format!("{name} / {guild}").into() };
    let v = &app.voice;
    let (title, color) = match v.status {
        VoiceStatus::Connected | VoiceStatus::Idle => (
            "Voice Connected",
            match v.quality() {
                Quality::Fair => Palette::YELLOW,
                Quality::Bad => Palette::RED,
                _ => Palette::GREEN,
            },
        ),
        VoiceStatus::Connecting => ("RTC Connecting", Palette::YELLOW),
        VoiceStatus::Reconnecting => ("Reconnecting", Palette::YELLOW),
        VoiceStatus::Failed => ("No Route", Palette::RED),
    };
    let ping: SharedString = match (v.ping_ms, &v.last_error) {
        (_, Some(e)) if v.status == VoiceStatus::Failed => e.clone().into(),
        (Some(p), _) => format!("Ping: {p} ms{}", if v.e2ee { " · End-to-end encrypted" } else { "" }).into(),
        (None, _) => "Waiting for the voice server".into(),
    };
    div()
        .flex_none()
        .bg(c(Palette::BG_SECONDARY_ALT))
        .border_b_1()
        .border_color(c(Palette::DIVIDER))
        .px(px(8.))
        .pt(px(8.))
        .pb(px(8.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .id("voice-status")
                        .flex_1()
                        .min_w_0()
                        .pl(px(4.))
                        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(ping.clone()).build(window, cx))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(signal_bars(c(color)))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(c(color))
                                        .child(title),
                                )
                                .when(v.e2ee, |d| d.child(asset_icon(SHIELD, 12., c(Palette::GREEN)))),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(c(Palette::TEXT_MUTED))
                                .text_ellipsis()
                                .child(label),
                        ),
                )
                .child(
                    icon_button("hangup", Icon::PhoneHangup, c(Palette::INTERACTIVE), 20.)
                        .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Disconnect").build(window, cx))
                        .on_click(cx.listener(|a, _, _, cx| a.join_voice(None, cx))),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(action_button("voice-camera", CAMERA, "Turn On Camera", cx))
                .child(action_button("voice-screen", SCREEN, "Share Your Screen", cx))
                .child(action_button("voice-activity", ACTIVITY, "Start An Activity", cx))
                .child(action_button("voice-soundboard", SOUNDBOARD, "Open Soundboard", cx)),
        )
}
