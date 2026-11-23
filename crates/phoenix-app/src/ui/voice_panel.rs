//! "Voice Connected" panel above the user panel, and the speaking ring used
//! on voice channel member avatars.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Hsla, IntoElement, MouseButton, ParentElement, Pixels, Point,
    SharedString, Styled, Window, div, prelude::*, px, svg,
};

use crate::app::PhoenixApp;
use crate::assets::Icon;
use crate::theme::{Palette, c, z};
use crate::model::Id;
use crate::ui::context_menu::{self, MenuItem};
use crate::ui::{avatar, icon, icon_button};
use crate::voice::prefs::{prefs, update};
use crate::voice::{Quality, VoiceStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Connection,
    Privacy,
}

thread_local! {
    /// Right-click menu on a voice channel member.
    static USER_MENU: Cell<Option<(Id, Point<Pixels>)>> = const { Cell::new(None) };
    /// "Voice Connected" popout.
    static POPOUT: Cell<Option<Tab>> = const { Cell::new(None) };
}

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
        .h(z(32.))
        .rounded(z(8.))
        .bg(c(Palette::BG_MOD_HOVER))
        .hover(|s| s.bg(c(Palette::BG_MOD_SELECTED)))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .child(asset_icon(path, 20. * crate::theme::zoom(), c(Palette::INTERACTIVE)))
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
    // Sits inside the sidebar's floating panels card, separated from the user
    // row by a 1px subtle border.
    div()
        .flex_none()
        .border_b_1()
        .border_color(c(Palette::PANEL_BORDER))
        .p(z(8.))
        .flex()
        .flex_col()
        .gap(z(8.))
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
                        .pl(z(4.))
                        .cursor_pointer()
                        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(ping.clone()).build(window, cx))
                        .on_click(cx.listener(|_, _, _, cx| {
                            POPOUT.with(|p| p.set(if p.get().is_some() { None } else { Some(Tab::Connection) }));
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(signal_bars(c(color)))
                                .child(
                                    div()
                                        .text_size(z(14.))
                                        .line_height(z(20.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(c(color))
                                        .child(title),
                                )
                                .when(v.e2ee, |d| {
                                    d.child(
                                        div()
                                            .id("voice-e2ee")
                                            .cursor_pointer()
                                            .child(asset_icon(SHIELD, 12., c(Palette::GREEN)))
                                            .tooltip(|window, cx| {
                                                gpui_component::tooltip::Tooltip::new("End-to-end encrypted").build(window, cx)
                                            })
                                            .on_click(cx.listener(|_, _, _, cx| {
                                                cx.stop_propagation();
                                                POPOUT.with(|p| p.set(Some(Tab::Privacy)));
                                                cx.notify();
                                            })),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_size(z(12.))
                                .line_height(z(16.))
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
                .gap(z(8.))
                .child(action_button("voice-camera", CAMERA, "Turn On Camera", cx))
                .child(action_button("voice-screen", SCREEN, "Share Your Screen", cx))
                .child(action_button("voice-activity", ACTIVITY, "Start An Activity", cx))
                .child(action_button("voice-soundboard", SOUNDBOARD, "Open Soundboard", cx)),
        )
}

/// Voice channel member row in the channel list: speaking ring, local
/// mute / deafen badges, right-click menu with the user volume.
pub fn voice_user_row(app: &PhoenixApp, entity: &Entity<PhoenixApp>, user: Id) -> AnyElement {
    let u = app.users.get(&user);
    let me = user == app.me.id;
    let in_call = app.voice_channel.is_some_and(|ch| app.channels.get(&ch).is_some_and(|c| c.voice_users.contains(&user)));
    let local_muted = !me && prefs().lock().user(user).muted;
    let speaking = app.voice.is_speaking(user) && !local_muted;
    let e = entity.clone();
    div()
        .ml(px(36.))
        .mr(px(8.))
        .h(px(32.))
        .px(px(8.))
        .rounded(px(4.))
        .flex()
        .items_center()
        .gap(px(8.))
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .child(speaking_ring(avatar(u, 24.), 24., speaking))
        .child(
            div()
                .flex_1()
                .text_size(px(14.))
                .text_color(c(if speaking { Palette::TEXT_HEADER } else { Palette::TEXT_MUTED }))
                .text_ellipsis()
                .child(u.map(|u| u.display_name.clone()).unwrap_or_default()),
        )
        .when(local_muted, |d| d.child(icon(Icon::MicOff, 16., c(Palette::RED))))
        .when(me && app.deafened, |d| d.child(icon(Icon::HeadphonesOff, 16., c(Palette::TEXT_MUTED))))
        .when(me && app.mic_muted && !app.deafened, |d| d.child(icon(Icon::MicOff, 16., c(Palette::TEXT_MUTED))))
        .when(in_call, |d| {
            d.on_mouse_down(MouseButton::Right, move |ev, _, cx| {
                USER_MENU.with(|m| m.set(Some((user, ev.position))));
                e.update(cx, |_, cx| cx.notify());
            })
        })
        .into_any_element()
}

fn check_row(
    id: &'static str,
    label: &'static str,
    on: bool,
    f: impl Fn(&mut PhoenixApp) + 'static,
    cx: &mut Context<PhoenixApp>,
) -> AnyElement {
    div()
        .id(id)
        .h(px(32.))
        .px(px(8.))
        .my(px(2.))
        .rounded(px(2.))
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .text_size(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(0xB5BAC1))
        .hover(|s| s.bg(c(0x4752C4)).text_color(gpui::white()))
        .child(label)
        .child(
            div()
                .size(px(18.))
                .rounded(px(4.))
                .border_1()
                .border_color(c(if on { Palette::BLURPLE } else { 0xB5BAC1 }))
                .when(on, |d| d.bg(c(Palette::BLURPLE)))
                .flex()
                .items_center()
                .justify_center()
                .when(on, |d| d.child(div().size(px(8.)).rounded(px(1.)).bg(gpui::white()))),
        )
        .on_click(cx.listener(move |a, _, _, cx| {
            f(a);
            cx.notify();
        }))
        .into_any_element()
}

/// Voice items of a user's context menu while in a call (Discord: "User
/// Volume" slider and local "Mute" for others; Mute / Deafen for yourself).
/// Other menus (member list, user menu) can append these.
pub fn user_menu_items(app: &PhoenixApp, user: Id, cx: &mut Context<PhoenixApp>) -> Vec<MenuItem> {
    let mut items = Vec::new();
    if user == app.me.id {
        items.push(MenuItem::Custom(check_row(
            "vm-self-mute",
            "Mute",
            app.mic_muted,
            |a| {
                a.mic_muted = !a.mic_muted;
                crate::voice::sync_self_state(a);
            },
            cx,
        )));
        items.push(MenuItem::Custom(check_row(
            "vm-self-deafen",
            "Deafen",
            app.deafened,
            |a| {
                a.deafened = !a.deafened;
                crate::voice::sync_self_state(a);
            },
            cx,
        )));
        let e = cx.entity();
        items.push(MenuItem::Separator);
        items.push(MenuItem::action("Voice Settings", None, move |_, _, cx| {
            e.update(cx, |a, cx| {
                crate::ui::settings::open(a, cx);
                if let Some(s) = a.settings.as_mut() {
                    s.page = crate::ui::settings::Page::VoiceVideo;
                }
            });
        }));
        return items;
    }
    let ua = prefs().lock().user(user);
    let vol = ua.volume;
    items.push(MenuItem::Custom(
        div()
            .px(px(8.))
            .py(px(6.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(div().text_size(px(14.)).font_weight(FontWeight::MEDIUM).text_color(c(0xB5BAC1)).child("User Volume"))
            .child(crate::ui::settings::voice::slider(
                "user-volume",
                vol as f32 / 200.,
                format!("{vol}%").into(),
                None,
                Rc::new(move |_, f, done, _| {
                    let v = (f * 200.).round() as u32;
                    update(move |p| p.users.entry(user.0).or_default().volume = v, done);
                }),
                cx,
            ))
            .into_any_element(),
    ));
    items.push(MenuItem::Separator);
    items.push(MenuItem::Custom(check_row(
        "vm-user-mute",
        "Mute",
        ua.muted,
        move |_| {
            update(
                |p| {
                    let u = p.users.entry(user.0).or_default();
                    u.muted = !u.muted;
                },
                true,
            );
        },
        cx,
    )));
    items
}

fn user_menu(app: &PhoenixApp, user: Id, pos: Point<Pixels>, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let e = cx.entity();
    let dismiss: context_menu::Dismiss = Rc::new(move |_: &mut Window, cx: &mut App| {
        USER_MENU.with(|m| m.set(None));
        e.update(cx, |_, cx| cx.notify());
    });
    context_menu::render(pos, user_menu_items(app, user, cx), dismiss)
}

fn ping_graph(history: &[u32]) -> impl IntoElement {
    let n = crate::voice::PING_HISTORY;
    let max = history.iter().copied().max().unwrap_or(0).max(100) as f32;
    let off = n - history.len().min(n);
    div()
        .h(px(80.))
        .p(px(8.))
        .rounded(px(4.))
        .bg(c(Palette::BG_TERTIARY))
        .flex()
        .items_end()
        .gap(px(2.))
        .children((0..n).map(|i| {
            let p = i.checked_sub(off).and_then(|j| history.get(j)).copied();
            let col = match p {
                Some(p) if p < 150 => Palette::GREEN,
                Some(p) if p < 400 => Palette::YELLOW,
                Some(_) => Palette::RED,
                None => Palette::BG_MOD_HOVER,
            };
            let h = p.map(|p| (p as f32 / max * 64.).max(2.)).unwrap_or(2.);
            div().flex_1().h(px(h)).rounded(px(1.)).bg(c(col))
        }))
}

fn stat(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .gap(px(8.))
        .text_size(px(14.))
        .line_height(px(20.))
        .child(div().flex_none().text_color(c(Palette::TEXT_MUTED)).child(label))
        .child(div().min_w_0().text_ellipsis().font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_NORMAL)).child(value))
}

fn dash<T: ToString>(v: Option<T>) -> String {
    v.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

fn privacy_body(v: &crate::voice::VoiceUi) -> AnyElement {
    let groups: Vec<String> = v.privacy_code.as_deref().map(|c| c.split(' ').map(str::to_string).collect()).unwrap_or_default();
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(asset_icon(SHIELD, 20., c(if v.e2ee { Palette::GREEN } else { Palette::TEXT_MUTED })))
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(if v.e2ee { "End-to-end encrypted" } else { "Not end-to-end encrypted" }),
                ),
        )
        .child(div().text_size(px(14.)).line_height(px(18.)).text_color(c(Palette::TEXT_MUTED)).child(if v.e2ee {
            "Audio and video in this call are end-to-end encrypted. Compare the voice privacy code with the other participants: when it matches, nobody else can listen in."
        } else {
            "This call is protected by transport encryption only."
        }))
        .when(!groups.is_empty(), |d| {
            d.child(div().text_size(px(12.)).font_weight(FontWeight::BOLD).text_color(c(Palette::TEXT_MUTED)).child("VOICE PRIVACY CODE"))
                .child(div().flex().flex_wrap().gap(px(8.)).children(groups.into_iter().map(|g| {
                    div()
                        .px(px(8.))
                        .py(px(4.))
                        .rounded(px(4.))
                        .bg(c(Palette::BG_TERTIARY))
                        .font_family(crate::theme::FONT_MONO)
                        .text_size(px(16.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(g)
                })))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(c(Palette::TEXT_MUTED))
                        .child("The code changes whenever someone joins or leaves the call."),
                )
        })
        .into_any_element()
}

fn connection_body(v: &crate::voice::VoiceUi) -> AnyElement {
    let history: Vec<u32> = v.ping_history.iter().copied().collect();
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(ping_graph(&history))
        .child(stat("Average ping", dash(v.average_ping().map(|p| format!("{p} ms")))))
        .child(stat("Last ping", dash(v.ping_ms.map(|p| format!("{p} ms")))))
        .child(stat("Outbound packet loss rate", dash(v.stats.outbound_loss_pct.map(|l| format!("{l:.1}%")))))
        .child(stat("Inbound packet loss rate", format!("{:.1}%", v.inbound_loss_pct())))
        .child(stat("Packets sent / received", format!("{} / {}", v.stats.packets_sent, v.stats.packets_received)))
        .child(div().h(px(1.)).my(px(4.)).bg(c(Palette::DIVIDER)))
        .child(stat("Server", dash(v.endpoint.clone())))
        .child(stat("Encryption", dash(v.mode.clone())))
        .into_any_element()
}

fn popout(app: &PhoenixApp, tab: Tab, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let tab_button = |id: &'static str, label: &'static str, t: Tab, cx: &mut Context<PhoenixApp>| {
        let sel = t == tab;
        div()
            .id(id)
            .pb(px(6.))
            .border_b_2()
            .border_color(if sel { c(Palette::BLURPLE) } else { gpui::transparent_black() })
            .cursor_pointer()
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(if sel { Palette::TEXT_HEADER } else { Palette::TEXT_MUTED }))
            .hover(|s| s.text_color(c(Palette::TEXT_NORMAL)))
            .child(label)
            .on_click(cx.listener(move |_, _, _, cx| {
                POPOUT.with(|p| p.set(Some(t)));
                cx.notify();
            }))
    };
    let body = match tab {
        Tab::Connection => connection_body(&app.voice),
        Tab::Privacy => privacy_body(&app.voice),
    };
    let close = cx.listener(|_, _, _, cx| {
        POPOUT.with(|p| p.set(None));
        cx.notify();
    });
    gpui::deferred(
        div()
            .id("voice-popout-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .on_click(close)
            .child(
                div()
                    .id("voice-popout")
                    .absolute()
                    .left(px(80.))
                    .bottom(px(150.))
                    .w(px(320.))
                    .p(px(16.))
                    .rounded(px(8.))
                    .bg(c(Palette::BG_FLOATING))
                    .border_1()
                    .border_color(c(Palette::DIVIDER))
                    .shadow_lg()
                    .occlude()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .gap(px(16.))
                            .border_b_1()
                            .border_color(c(Palette::DIVIDER))
                            .child(tab_button("vp-tab-conn", "Connection", Tab::Connection, cx))
                            .child(tab_button("vp-tab-privacy", "Privacy", Tab::Privacy, cx)),
                    )
                    .child(body),
            ),
    )
    .with_priority(3)
    .into_any_element()
}

/// Root overlays owned by the voice UI (user menu, connection popout).
pub fn overlays(app: &PhoenixApp, _window: &mut Window, cx: &mut Context<PhoenixApp>) -> Vec<AnyElement> {
    let mut out = Vec::new();
    if app.voice_channel.is_none() {
        USER_MENU.with(|m| m.set(None));
        POPOUT.with(|p| p.set(None));
        return out;
    }
    if let Some(tab) = POPOUT.with(Cell::get) {
        out.push(popout(app, tab, cx));
    }
    if let Some((user, pos)) = USER_MENU.with(Cell::get) {
        out.push(user_menu(app, user, pos, cx));
    }
    out
}
