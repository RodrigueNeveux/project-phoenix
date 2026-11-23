//! User profile popout (click on a member / author), full profile modal and
//! the overlay host that also shows the status picker.

use crate::theme::z;
use gpui::{
    AnyElement, Context, Div, FontWeight, IntoElement, ParentElement, Pixels, Point, SharedString, Styled,
    Window, div, img, prelude::*, px,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    app::PhoenixApp,
    model::{Id, ImageRef},
    presence::{Activity, ActivityKind, Popout, Profile, format_date, status_label},
    theme::{DEFAULT_AVATAR, Palette, c, ca, role_color},
    ui::{avatar_with_status, members::app_tag, status_picker},
};

const POPOUT_W: f32 = 300.;
const POPOUT_H: f32 = 540.;
const MODAL_W: f32 = 600.;

impl PhoenixApp {
    /// Open the profile popout for `user` next to `at` and fetch the profile.
    pub fn open_profile(
        &mut self,
        user: Id,
        guild: Option<Id>,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.people.popout.as_ref().is_some_and(|p| p.user == user) {
            self.people.popout = None;
            cx.notify();
            return;
        }
        let note = cx.new(|cx| InputState::new(window, cx).placeholder("Click to add a note"));
        let sub = cx.subscribe_in(&note, window, move |this, input, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                let text = input.read(cx).value().to_string();
                let changed = this.people.profiles.get(&user).is_some_and(|p| p.note.as_ref() != text);
                if changed {
                    if let Some(p) = this.people.profiles.get_mut(&user) {
                        p.note = text.clone().into();
                    }
                    cx.background_spawn(this.backend.set_note(user, text)).detach();
                }
            }
        });
        self.people.status_picker = false;
        self.people.popout = Some(Popout { user, guild, at, note: note.clone(), _sub: sub });
        let fut = self.backend.user_profile(user, guild);
        cx.spawn_in(window, async move |this, cx| {
            let Ok(p) = fut.await else { return };
            let _ = this.update_in(cx, |a, window, cx| {
                note.update(cx, |n, cx| n.set_value(p.note.to_string(), window, cx));
                a.people.profiles.insert(user, p);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Popout -> full profile modal.
    pub fn open_profile_modal(&mut self, user: Id, cx: &mut Context<Self>) {
        self.people.popout = None;
        self.people.modal = Some(user);
        cx.notify();
    }

    /// "Message" button: open the DM and switch to it.
    pub fn message_user(&mut self, user: Id, cx: &mut Context<Self>) {
        self.people.close_overlays();
        let fut = self.backend.open_dm(user);
        cx.spawn(async move |this, cx| {
            let Ok(ch) = fut.await else { return };
            let _ = this.update(cx, |a, cx| a.select_channel(ch, cx));
        })
        .detach();
        cx.notify();
    }
}

/// Everything floating owned by this module, appended to the root.
pub fn overlays(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> Vec<AnyElement> {
    let mut out: Vec<AnyElement> = crate::ui::user_menu::render(app, cx).into_iter().collect();
    out.extend(crate::ui::nav_menus::render(app, cx));
    if !app.people.any_overlay() {
        return out;
    }
    let dim = app.people.modal.is_some();
    out.push(
        div()
            .id("people-backdrop")
            .absolute()
            .inset_0()
            .when(dim, |d| d.bg(ca(0x000000B3)))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|a, _, _, cx| {
                    a.people.close_overlays();
                    cx.notify();
                }),
            )
            .into_any_element(),
    );
    let vp = window.viewport_size();
    if let Some(p) = &app.people.popout {
        let x = (p.at.x - px(POPOUT_W + 16.)).max(z(8.));
        let y = (p.at.y - z(60.)).min(vp.height - px(POPOUT_H + 8.)).max(z(8.));
        out.push(
            div()
                .id("profile-popout")
                .absolute()
                .left(x)
                .top(y)
                .w(px(POPOUT_W))
                .occlude()
                .child(card(app, p.user, p.guild, Some(&p.note), false, cx))
                .into_any_element(),
        );
    }
    if let Some(u) = app.people.modal {
        let x = ((vp.width - px(MODAL_W)) / 2.).max(z(8.));
        out.push(
            div()
                .id("profile-modal")
                .absolute()
                .left(x)
                .top(z(64.))
                .w(px(MODAL_W))
                .occlude()
                .child(card(app, u, app.selected_guild, None, true, cx))
                .into_any_element(),
        );
    }
    if app.people.status_picker {
        out.push(status_picker::render(app, cx));
    }
    out
}

fn section_title(t: &'static str) -> Div {
    div()
        .text_size(z(12.))
        .font_weight(FontWeight::BOLD)
        .text_color(c(Palette::TEXT_HEADER))
        .mb(z(6.))
        .child(t)
}

fn activity_block(a: &Activity) -> Option<AnyElement> {
    let title = match a.kind {
        ActivityKind::Custom => return None,
        ActivityKind::Playing => "PLAYING A GAME".into(),
        ActivityKind::Streaming => format!("LIVE ON {}", a.name.to_uppercase()),
        ActivityKind::Listening => format!("LISTENING TO {}", a.name.to_uppercase()),
        ActivityKind::Watching => format!("WATCHING {}", a.name.to_uppercase()),
        ActivityKind::Competing => "COMPETING".into(),
    };
    let spotify = a.is_spotify();
    let line1 = if spotify || a.kind == ActivityKind::Streaming { a.details.clone() } else { Some(a.name.clone()) };
    let mut lines: Vec<SharedString> = Vec::new();
    if spotify {
        if let Some(s) = &a.state {
            lines.push(format!("by {s}").into());
        }
    } else {
        lines.extend(a.details.clone().filter(|_| a.kind != ActivityKind::Streaming));
        lines.extend(a.state.clone());
    }
    if let Some(start) = a.start {
        let mins = (chrono::Utc::now().timestamp_millis() - start).max(0) / 60_000;
        lines.push(if mins >= 60 { format!("for {}h {}m", mins / 60, mins % 60) } else { format!("for {mins}m") }.into());
    }
    let art: AnyElement = match &a.image {
        Some(u) => img(u.clone()).size(z(60.)).rounded(z(8.)).flex_none().into_any_element(),
        None => div().size(z(60.)).rounded(z(8.)).flex_none().bg(c(Palette::BG_MOD_SELECTED)).into_any_element(),
    };
    Some(
        div()
            .flex()
            .flex_col()
            .child(div().text_size(z(12.)).font_weight(FontWeight::BOLD).text_color(c(Palette::TEXT_HEADER)).mb(z(8.)).child(title))
            .child(
                div()
                    .flex()
                    .gap(z(10.))
                    .items_center()
                    .child(art)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .text_size(z(14.))
                            .text_color(c(Palette::TEXT_NORMAL))
                            .when_some(line1, |d, l| {
                                d.child(div().font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_HEADER)).text_ellipsis().child(l))
                            })
                            .children(lines.into_iter().map(|l| div().text_size(z(12.)).text_ellipsis().child(l))),
                    ),
            )
            .into_any_element(),
    )
}

fn card(
    app: &PhoenixApp,
    uid: Id,
    guild: Option<Id>,
    note: Option<&gpui::Entity<InputState>>,
    full: bool,
    cx: &mut Context<PhoenixApp>,
) -> Div {
    let user = app.users.get(&uid);
    let prof: Option<&Profile> = app.people.profiles.get(&uid);
    let presence = app.presence_of(uid);
    let acts = app.people.activities.get(&uid).map(Vec::as_slice).unwrap_or_default();
    let custom = if uid == app.me.id { app.people.custom_status.clone() } else { None }
        .or_else(|| acts.iter().find(|a| a.kind == ActivityKind::Custom).map(Activity::headline));
    let member = guild.and_then(|g| app.members.get(&g)).and_then(|ms| ms.iter().find(|m| m.user == uid));
    let nick = prof.and_then(|p| p.nick.clone()).or_else(|| {
        guild.and_then(|g| app.people.lists.get(&g)).and_then(|rows| {
            rows.iter().find_map(|r| match r {
                crate::presence::ListEntry::Member(m, n) if m.user == uid => n.clone(),
                _ => None,
            })
        })
    });
    let display = nick.or_else(|| user.map(|u| u.display_name.clone())).unwrap_or_else(|| "Unknown user".into());
    let username = user.map(|u| u.name.clone()).unwrap_or_default();
    let fallback_accent = match user.map(|u| &u.avatar) {
        Some(ImageRef::Default(i)) => DEFAULT_AVATAR[*i as usize % DEFAULT_AVATAR.len()],
        _ => Palette::BLURPLE,
    };
    let accent = prof.and_then(|p| p.accent).unwrap_or(fallback_accent);
    let banner_h = if full { 212. } else { 105. };
    let av = if full { 120. } else { 80. };
    let bg = Palette::BG_FLOATING;
    let mut role_ids: Vec<Id> = prof.map(|p| p.roles.clone()).filter(|r| !r.is_empty()).or_else(|| member.map(|m| m.roles.clone())).unwrap_or_default();
    let guild_roles = guild.and_then(|g| app.roles.get(&g));
    if let Some(gr) = guild_roles {
        role_ids.sort_by_key(|r| -gr.iter().find(|x| x.id == *r).map(|x| x.position).unwrap_or(0));
    }
    let roles: Vec<_> = role_ids
        .iter()
        .filter_map(|r| guild_roles?.iter().find(|x| x.id == *r).cloned())
        .collect();

    let banner = div()
        .h(px(banner_h))
        .w_full()
        .bg(c(accent))
        .rounded_t(z(8.))
        .overflow_hidden()
        .when_some(prof.and_then(|p| p.banner.clone()), |d, b| d.child(img(b).w_full().h(px(banner_h))));

    let avatar = div()
        .absolute()
        .left(z(16.))
        .top(px(banner_h - av / 2.))
        .p(px(if full { 7. } else { 6. }))
        .rounded_full()
        .bg(c(bg))
        .child(avatar_with_status(user, av, presence, c(bg)))
        .when_some(prof.and_then(|p| p.decoration.clone()), |d, deco| {
            let s = av * 1.2;
            d.child(img(deco).absolute().size(px(s)).left(px(6. - (s - av) / 2.)).top(px(6. - (s - av) / 2.)))
        });

    let e = cx.entity();
    let mut body = div()
        .px(z(16.))
        .pt(px(av / 2. + 16.))
        .pb(z(16.))
        .flex()
        .flex_col()
        .gap(z(12.))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(z(6.))
                        .child(
                            div()
                                .text_size(z(20.))
                                .line_height(z(24.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .text_ellipsis()
                                .child(display),
                        )
                        .when(user.is_some_and(|u| u.bot), |d| d.child(app_tag())),
                )
                .child(
                    div()
                        .flex()
                        .gap(z(4.))
                        .text_size(z(14.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(username)
                        .when_some(prof.map(|p| p.pronouns.clone()).filter(|p| !p.is_empty()), |d, p| {
                            d.child("•").child(p)
                        }),
                )
                .when(uid == app.me.id, |d| {
                    d.child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(status_label(presence)))
                }),
        )
        .when_some(custom, |d, t| {
            d.child(
                div()
                    .px(z(10.))
                    .py(z(6.))
                    .rounded(z(8.))
                    .bg(c(Palette::BG_SECONDARY))
                    .text_size(z(14.))
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child(t),
            )
        });

    let mut inner = div().p(z(12.)).rounded(z(8.)).bg(c(Palette::BG_SECONDARY)).flex().flex_col().gap(z(14.));
    match prof {
        None => {
            inner = inner.child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child("Loading profile…"));
        }
        Some(p) => {
            if !p.bio.is_empty() {
                inner = inner.child(
                    div()
                        .child(section_title("About Me"))
                        .child(div().text_size(z(14.)).line_height(z(18.)).text_color(c(Palette::TEXT_NORMAL)).child(p.bio.clone())),
                );
            }
            let since = div()
                .flex()
                .items_center()
                .gap(z(8.))
                .text_size(z(14.))
                .text_color(c(Palette::TEXT_NORMAL))
                .child(format_date(p.created))
                .when_some(p.joined, |d, j| {
                    d.child(div().size(z(4.)).rounded_full().bg(c(Palette::INTERACTIVE_MUTED))).child(format_date(j))
                });
            inner = inner.child(div().child(section_title("Member Since")).child(since));
            if full && p.mutual_guilds > 0 {
                inner = inner.child(
                    div().text_size(z(14.)).text_color(c(Palette::TEXT_MUTED)).child(format!("{} Mutual Servers", p.mutual_guilds)),
                );
            }
        }
    }
    for a in acts {
        if let Some(b) = activity_block(a) {
            inner = inner.child(b);
        }
    }
    if guild.is_some() && !roles.is_empty() {
        let label = if roles.len() == 1 { "Role" } else { "Roles" };
        inner = inner.child(
            div()
                .child(div().text_size(z(12.)).font_weight(FontWeight::BOLD).text_color(c(Palette::TEXT_HEADER)).mb(z(6.)).child(label))
                .child(div().flex().flex_wrap().gap(z(4.)).children(roles.into_iter().map(|r| {
                    div()
                        .flex()
                        .items_center()
                        .gap(z(4.))
                        .h(z(24.))
                        .px(z(6.))
                        .rounded(z(4.))
                        .bg(c(Palette::BG_TERTIARY))
                        .text_size(z(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(div().size(z(12.)).rounded_full().bg(role_color(r.color.or(Some(0x99AAB5)))))
                        .child(r.name.clone())
                }))),
        );
    }
    if let Some(n) = note {
        inner = inner.child(
            div()
                .child(section_title("Note"))
                .child(div().text_size(z(12.)).child(Input::new(n).appearance(false).bordered(false))),
        );
    }
    body = body.child(inner);

    if uid != app.me.id {
        let e2 = e.clone();
        body = body.child(
            div()
                .id(("profile-message", uid.0))
                .h(z(40.))
                .px(z(12.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .cursor_pointer()
                .bg(c(if full { Palette::BLURPLE } else { Palette::BG_SECONDARY }))
                .border_1()
                .border_color(c(Palette::DIVIDER))
                .text_size(z(14.))
                .text_color(c(if full { 0xFFFFFF } else { Palette::TEXT_MUTED }))
                .hover(|s| s.border_color(c(Palette::INTERACTIVE_MUTED)))
                .on_click(move |_, _, cx| e2.update(cx, |a, cx| a.message_user(uid, cx)))
                .child(if full { "Send Message".to_owned() } else { format!("Message @{}", user.map(|u| u.name.as_ref()).unwrap_or("user")) }),
        );
    }
    if !full {
        body = body.child(
            div()
                .id(("profile-full", uid.0))
                .text_size(z(12.))
                .text_color(c(Palette::TEXT_MUTED))
                .cursor_pointer()
                .hover(|s| s.text_color(c(Palette::TEXT_NORMAL)))
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.open_profile_modal(uid, cx)))
                .child("View Full Profile"),
        );
    }

    div()
        .relative()
        .w_full()
        .rounded(z(8.))
        .bg(c(bg))
        .shadow_lg()
        .border_1()
        .border_color(c(Palette::PANEL_BORDER))
        .child(banner)
        .child(avatar)
        .child(body)
}
