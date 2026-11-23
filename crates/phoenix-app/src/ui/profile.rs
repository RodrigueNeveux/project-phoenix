//! User profile popout (click on a member / author), full profile modal and
//! the overlay host that also shows the status picker.

use crate::theme::{generated::idx, tok, z};
use gpui::{
    AnyElement, Context, Div, FontWeight, IntoElement, ParentElement, Pixels, Point, SharedString, Styled,
    Window, div, img, prelude::*, px,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    app::PhoenixApp,
    model::{Id, ImageRef},
    presence::{Activity, ActivityKind, Popout, Profile, format_date, status_label},
    theme::{DEFAULT_AVATAR, Palette, c, role_color},
    ui::{avatar_with_status, members::app_tag, status_picker},
};

// 253781 `:root` --custom-user-profile-popout-width
const POPOUT_W: f32 = 300.;
// Estimated popout height, only used to keep it on screen
// (253781 `outer_P1YEjQ.user-profile-popout` max-height: 100vh - 20px).
const POPOUT_H: f32 = 540.;
// f4f30b `root_ZhNJfE` --custom-modal-v2-width (min-width 929px)
const MODAL_W: f32 = 962.;
// f4f30b `root_ZhNJfE` --custom-modal-v2-height: min(800px, 100vh - 2*--space-48)
const MODAL_H: f32 = 800.;

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
        let note = cx.new(|cx| InputState::new(window, cx).placeholder(crate::t!("profile.note_placeholder")));
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

    /// crate::t!("profile.message") button: open the DM and switch to it.
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
            // modal layer backdrop: --background-scrim
            .when(dim, |d| d.bg(c(tok(idx::BACKGROUND_SCRIM))))
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
                // outer_P1YEjQ.user-profile-popout max-height: calc(100vh - 20px)
                .max_h(vp.height - y - px(10.))
                .flex()
                .flex_col()
                .occlude()
                .child(popout_card(app, p.user, p.guild, Some(&p.note), cx))
                .into_any_element(),
        );
    }
    if let Some(u) = app.people.modal {
        // root_ZhNJfE: width min(962px, 100vw - 2*--space-48),
        // height min(800px, 100vh - 2*--space-48)
        let w = px(MODAL_W).min(vp.width - px(96.));
        let h = px(MODAL_H).min(vp.height - px(96.));
        out.push(
            div()
                .id("profile-modal")
                .absolute()
                .left(((vp.width - w) / 2.).max(px(0.)))
                .top(((vp.height - h) / 2.).max(px(0.)))
                .w(w)
                .h(h)
                .occlude()
                .child(modal_card(app, u, app.selected_guild, vp.width, cx))
                .into_any_element(),
        );
    }
    if app.people.status_picker {
        out.push(status_picker::render(app, cx));
    }
    out
}

/// Section heading inside a profile card (Text heading-sm/semibold, 12px/16px).
pub(crate) fn section_title(t: SharedString) -> Div {
    div()
        .text_size(z(12.))
        .line_height(z(16.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(tok(idx::TEXT_STRONG)))
        .mb(z(4.))
        .child(t)
}

/// overlay_P1YEjQ (bg --user-profile-overlay-background, --radius-sm) with
/// card_NOlkhE padding 12px.
fn overlay_card() -> Div {
    div()
        .flex()
        .flex_col()
        .p(z(12.)) // card_NOlkhE padding
        .rounded(z(8.)) // overlay_P1YEjQ --radius-sm
        .bg(c(tok(idx::USER_PROFILE_OVERLAY_BACKGROUND)))
}

fn activity_block(a: &Activity) -> Option<AnyElement> {
    let title = match a.kind {
        ActivityKind::Custom => return None,
        ActivityKind::Playing => crate::t!("activity.playing").to_string(),
        ActivityKind::Streaming => crate::t!("activity.streaming", name = a.name).to_string(),
        ActivityKind::Listening => crate::t!("activity.listening", name = a.name).to_string(),
        ActivityKind::Watching => crate::t!("activity.watching", name = a.name).to_string(),
        ActivityKind::Competing => crate::t!("activity.competing").to_string(),
    };
    let spotify = a.is_spotify();
    let line1 = if spotify || a.kind == ActivityKind::Streaming { a.details.clone() } else { Some(a.name.clone()) };
    let mut lines: Vec<SharedString> = Vec::new();
    if spotify {
        if let Some(s) = &a.state {
            lines.push(crate::t!("activity.by", s = s));
        }
    } else {
        lines.extend(a.details.clone().filter(|_| a.kind != ActivityKind::Streaming));
        lines.extend(a.state.clone());
    }
    if let Some(start) = a.start {
        let mins = (chrono::Utc::now().timestamp_millis() - start).max(0) / 60_000;
        lines.push(if mins >= 60 { format!("{}:{:02}:00", mins / 60, mins % 60) } else { format!("{mins}:00") }.into());
    }
    // thumbnailContainer (member list Activity card) 72px -> profile
    // activity art uses --radius-sm.
    let art: AnyElement = match &a.image {
        Some(u) => img(u.clone()).size(z(72.)).rounded(z(8.)).flex_none().into_any_element(),
        None => div().size(z(72.)).rounded(z(8.)).flex_none().bg(c(tok(idx::BACKGROUND_MOD_NORMAL))).into_any_element(),
    };
    Some(
        overlay_card()
            .child(section_title_owned(title))
            .child(
                div()
                    .flex()
                    .gap(z(12.)) // --space-12
                    .items_center()
                    .child(art)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .text_size(z(14.))
                            .line_height(z(18.))
                            .text_color(c(tok(idx::TEXT_DEFAULT)))
                            .when_some(line1, |d, l| {
                                d.child(div().font_weight(FontWeight::SEMIBOLD).text_color(c(tok(idx::TEXT_STRONG))).text_ellipsis().child(l))
                            })
                            .children(lines.into_iter().map(|l| div().text_size(z(12.)).line_height(z(16.)).text_ellipsis().child(l))),
                    ),
            )
            .into_any_element(),
    )
}

fn section_title_owned(t: String) -> Div {
    div()
        .text_size(z(12.))
        .line_height(z(16.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(tok(idx::TEXT_STRONG)))
        .mb(z(8.))
        .child(t)
}

/// Everything both layouts need about one user.
pub(crate) struct Info<'a> {
    pub(crate) user: Option<&'a crate::model::User>,
    pub(crate) prof: Option<&'a Profile>,
    pub(crate) presence: crate::model::Presence,
    pub(crate) acts: &'a [Activity],
    pub(crate) custom: Option<SharedString>,
    pub(crate) display: SharedString,
    pub(crate) username: SharedString,
    pub(crate) accent: u32,
    pub(crate) roles: Vec<crate::model::Role>,
}

pub(crate) fn info<'a>(app: &'a PhoenixApp, uid: Id, guild: Option<Id>) -> Info<'a> {
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
    let display = nick.or_else(|| user.map(|u| u.display_name.clone())).unwrap_or_else(|| crate::t!("common.unknown_user"));
    let username = user.map(|u| u.name.clone()).unwrap_or_default();
    let fallback_accent = match user.map(|u| &u.avatar) {
        Some(ImageRef::Default(i)) => DEFAULT_AVATAR[*i as usize % DEFAULT_AVATAR.len()],
        _ => Palette::BLURPLE,
    };
    let accent = prof.and_then(|p| p.accent).unwrap_or(fallback_accent);
    let mut role_ids: Vec<Id> =
        prof.map(|p| p.roles.clone()).filter(|r| !r.is_empty()).or_else(|| member.map(|m| m.roles.clone())).unwrap_or_default();
    let guild_roles = guild.and_then(|g| app.roles.get(&g));
    if let Some(gr) = guild_roles {
        role_ids.sort_by_key(|r| -gr.iter().find(|x| x.id == *r).map(|x| x.position).unwrap_or(0));
    }
    let roles = role_ids.iter().filter_map(|r| guild_roles?.iter().find(|x| x.id == *r).cloned()).collect();
    Info { user, prof, presence, acts, custom, display, username, accent, roles }
}

/// Banner: 58495 `banner_s9bWOi` (radius --radius-sm/--radius-lg top, 1px
/// --border-muted bottom line), height --custom-user-profile-banner-height.
pub(crate) fn banner(i: &Info, h: f32, radius: f32) -> Div {
    div()
        .h(z(h))
        .w_full()
        .flex_none()
        .bg(c(i.accent))
        .rounded_t(z(radius))
        .overflow_hidden()
        .border_b_1()
        .border_color(c(tok(idx::BORDER_MUTED)))
        .when_some(i.prof.and_then(|p| p.banner.clone()), |d, b| d.child(img(b).w_full().h(z(h))))
}

/// Avatar with its cut-out ring and decoration. 875762 `avatar_WRLIGC`
/// positions it; the ring is the card background (6px at 80px, 7px at 120px).
pub(crate) fn avatar_block(i: &Info, size: f32, ring: f32, ring_color: u32) -> Div {
    div()
        .p(z(ring))
        .rounded_full()
        .bg(c(ring_color))
        .child(avatar_with_status(i.user, size, i.presence, c(ring_color)))
        .when_some(i.prof.and_then(|p| p.decoration.clone()), |d, deco| {
            // Avatar decorations are drawn at 1.2x the avatar, centred.
            let s = size * 1.2;
            d.child(img(deco).absolute().size(z(s)).left(z(ring - (s - size) / 2.)).top(z(ring - (s - size) / 2.)))
        })
}

/// Badge icons. Popout: 352421 `headerTag_NOlkhE` (bottom 8px, end 16px,
/// --radius-xs); the badge list bundle itself isn't cached, icons are drawn
/// at the 22px of the CDN badge assets with --space-4 between them.
fn badges(i: &Info) -> Option<Div> {
    let b = i.prof.map(|p| p.badges.as_slice()).unwrap_or_default();
    (!b.is_empty()).then(|| {
        div()
            .flex()
            .flex_wrap()
            .gap(z(4.))
            .p(z(2.))
            .rounded(z(4.)) // --radius-xs
            .children(b.iter().map(|(_, icon)| img(icon.clone()).size(z(22.)).flex_none()))
    })
}

/// Custom status thought bubble (82171 `outer_rmCN9_` / `inner_rmCN9_`).
fn status_bubble(t: SharedString) -> Div {
    div()
        .max_w(z(181.)) // outer_rmCN9_ max-width
        .min_w(z(42.))
        .px(z(12.)) // inner_rmCN9_ padding 8px 12px
        .py(z(8.))
        .rounded(z(16.)) // --radius-lg
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGHEST))) // --custom-status-bubble-background-color
        .border_1()
        .border_color(c(tok(idx::BORDER_MUTED)))
        .shadow_sm() // --shadow-low
        .text_size(z(14.))
        .line_height(z(18.))
        .text_color(c(tok(idx::TEXT_DEFAULT))) // content_rmCN9_
        .line_clamp(2)
        .child(t)
}

/// Name block (866212 `displayNameRow_C2XpHA`, `usernameAndPronounsRow_`).
pub(crate) fn name_block(i: &Info, big: bool) -> Div {
    let tag = i.user.and_then(|u| u.tag.clone());
    let pronouns = i.prof.map(|p| p.pronouns.clone()).filter(|p| !p.is_empty());
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(4.)) // displayNameRow gap --space-4
                .child(
                    div()
                        .text_size(z(if big { 24. } else { 20. }))
                        .line_height(z(if big { 30. } else { 24. })) // line-height 24px / .lg 30px
                        .font_weight(FontWeight::BOLD)
                        .text_color(c(tok(idx::TEXT_STRONG)))
                        .text_ellipsis()
                        .child(i.display.clone()),
                )
                .when(i.user.is_some_and(|u| u.bot), |d| d.child(app_tag())),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                // column-gap 9.33px, 6px with pronouns; row-gap 2px (popout)
                .gap_x(z(if pronouns.is_some() { 6. } else { 9.33 }))
                .gap_y(z(2.))
                .text_size(z(14.))
                .line_height(z(18.))
                .child(div().h(z(24.)).flex().items_center().text_color(c(tok(idx::TEXT_STRONG))).child(i.username.clone()))
                .when_some(pronouns, |d, p| {
                    // dotSpacer_C2XpHA: 4px round --text-default, margin-inline 1px
                    d.child(div().size(z(4.)).mx(z(1.)).rounded_full().bg(c(tok(idx::TEXT_DEFAULT))))
                        .child(div().mr(z(2.)).text_ellipsis().text_color(c(tok(idx::TEXT_DEFAULT))).child(p))
                })
                .when_some(tag, |d, (t, badge)| d.child(guild_tag_pill(t, badge))),
        )
}

/// 866212 `guildTagPill_C2XpHA`: 24px, --radius-sm, --background-mod-subtle,
/// 1px --border-muted, padding-inline --space-8; hover --background-mod-strong.
fn guild_tag_pill(t: SharedString, badge: Option<SharedString>) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("guild-tag-{t}")))
        .h(z(24.))
        .px(z(8.))
        .flex()
        .items_center()
        .gap(z(4.))
        .rounded(z(8.))
        .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
        .border_1()
        .border_color(c(tok(idx::BORDER_MUTED)))
        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_STRONG))))
        .text_size(z(14.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(tok(idx::TEXT_STRONG)))
        .when_some(badge, |d, b| d.child(img(b).size(z(14.)).flex_none()))
        .child(t)
}

/// Role pills (`role_nie4FD`: 22px, --radius-xs 4px, --background-mod-normal,
/// 12px/500, padding 4px, margin 0 4px 4px 0; `roleCircle_7RsUnm` 12px, margin 0 4px).
fn roles_block(roles: Vec<crate::model::Role>) -> Div {
    let label = if roles.len() == 1 { crate::t!("profile.role") } else { crate::t!("profile.roles") };
    div().child(section_title(label)).child(div().flex().flex_wrap().children(roles.into_iter().map(|r| {
        div()
            .flex()
            .items_center()
            .h(z(22.))
            .p(z(4.))
            .mr(z(4.))
            .mb(z(4.))
            .rounded(z(4.))
            .bg(c(tok(idx::BACKGROUND_MOD_NORMAL)))
            .text_size(z(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(tok(idx::INTERACTIVE_TEXT_ACTIVE)))
            .child(div().size(z(12.)).mx(z(4.)).flex_none().rounded_full().bg(role_color(r.color.or(Some(0x99AAB5)))))
            .child(div().mr(z(4.)).max_w(z(200.)).text_ellipsis().child(r.name.clone())) // roleName_nie4FD
    })))
}

/// About me / member since / roles / note sections, shared by both layouts.
fn details(i: &Info, guild: Option<Id>, note: Option<&gpui::Entity<InputState>>, mutuals: bool) -> Vec<AnyElement> {
    let mut out: Vec<AnyElement> = Vec::new();
    let Some(p) = i.prof else {
        out.push(div().text_size(z(12.)).text_color(c(tok(idx::TEXT_MUTED))).child(crate::t!("profile.loading")).into_any_element());
        return out;
    };
    if !p.bio.is_empty() {
        out.push(
            div()
                .text_size(z(14.))
                .line_height(z(18.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .child(p.bio.clone())
                .into_any_element(),
        );
    }
    let since = div()
        .flex()
        .items_center()
        .gap(z(8.))
        .text_size(z(14.))
        .line_height(z(18.))
        .text_color(c(tok(idx::TEXT_DEFAULT)))
        .child(format_date(p.created))
        .when_some(p.joined, |d, j| {
            d.child(div().size(z(4.)).rounded_full().bg(c(tok(idx::TEXT_DEFAULT)))).child(format_date(j))
        });
    out.push(div().child(section_title(crate::t!("profile.member_since"))).child(since).into_any_element());
    if mutuals && p.mutual_guilds > 0 {
        out.push(
            div()
                .text_size(z(14.))
                .text_color(c(tok(idx::TEXT_MUTED)))
                .child(crate::t!("profile.mutual_servers", n = p.mutual_guilds))
                .into_any_element(),
        );
    }
    if guild.is_some() && !i.roles.is_empty() {
        out.push(roles_block(i.roles.clone()).into_any_element());
    }
    if let Some(n) = note {
        out.push(
            div()
                .child(section_title(crate::t!("profile.note")))
                .child(div().text_size(z(12.)).child(Input::new(n).appearance(false).bordered(false)))
                .into_any_element(),
        );
    }
    out
}

/// Mana button (384209 `button_F7qvhY`): --radius-sm, 1px border; `sm`
/// 30px inner + border = 32px with padding 4px 12px, `md` 40px with 8px 16px.
pub fn mana_button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, primary: bool, md: bool) -> gpui::Stateful<Div> {
    let (bg, hover, fg, border) = if primary {
        (
            idx::CONTROL_PRIMARY_BACKGROUND_DEFAULT,
            idx::CONTROL_PRIMARY_BACKGROUND_HOVER,
            idx::CONTROL_PRIMARY_TEXT_DEFAULT,
            idx::CONTROL_PRIMARY_BORDER_DEFAULT,
        )
    } else {
        (
            idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT,
            idx::CONTROL_SECONDARY_BACKGROUND_HOVER,
            idx::CONTROL_SECONDARY_TEXT_DEFAULT,
            idx::CONTROL_SECONDARY_BORDER_DEFAULT,
        )
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(z(if md { 40. } else { 32. }))
        .min_w(z(if md { 100. } else { 60. })) // --__button-min-width
        .px(z(if md { 16. } else { 12. }))
        .rounded(z(8.))
        .border_1()
        .border_color(c(tok(border)))
        .bg(c(tok(bg)))
        .hover(move |s| s.bg(c(tok(hover))))
        .cursor_pointer()
        .text_size(z(14.))
        .line_height(z(16.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(fg)))
        .whitespace_nowrap()
        .child(label.into())
}

fn popout_card(
    app: &PhoenixApp,
    uid: Id,
    guild: Option<Id>,
    note: Option<&gpui::Entity<InputState>>,
    cx: &mut Context<PhoenixApp>,
) -> Div {
    let i = info(app, uid, guild);
    let bg = tok(idx::BACKGROUND_SURFACE_HIGH); // outer_P1YEjQ background
    let e = cx.entity();
    // header_NOlkhE: min-height banner (105) + 35px; avatar_WRLIGC popout
    // inset-inline-start 16px, top 61px (6px ring around the 80px avatar).
    let header = div()
        .relative()
        .flex_none()
        .min_h(z(105. + 35.))
        .child(banner(&i, 105., 8.))
        .child(div().absolute().left(z(16.)).top(z(61.)).child(avatar_block(&i, 80., 6., bg)))
        .when_some(badges(&i), |d, b| d.child(div().absolute().right(z(16.)).bottom(z(8.)).child(b)))
        // container_rmCN9_ popout: inset-inline-start 105px, top banner - 7px
        .when_some(i.custom.clone(), |d, t| d.child(div().absolute().left(z(105.)).top(z(98.)).child(status_bubble(t))));

    // body_NOlkhE: column, gap 12px, padding 4px 16px 8px
    let mut body = div()
        .id(("profile-body", uid.0))
        .flex()
        .flex_col()
        .gap(z(12.))
        .pt(z(4.))
        .px(z(16.))
        .pb(z(8.))
        .min_h_0()
        .overflow_y_scroll()
        .child(name_block(&i, false))
        .when(uid == app.me.id, |d| {
            d.child(div().text_size(z(12.)).text_color(c(tok(idx::TEXT_MUTED))).child(status_label(i.presence)))
        })
        .children(details(&i, guild, note, false));
    for a in i.acts {
        if let Some(b) = activity_block(a) {
            body = body.child(b);
        }
    }
    let e2 = e.clone();
    body = body.child(
        div()
            .id(("profile-full", uid.0))
            .text_size(z(12.))
            .text_color(c(tok(idx::TEXT_LINK)))
            .cursor_pointer()
            .hover(|s| s.underline())
            .on_click(move |_, _, cx| e2.update(cx, |a, cx| a.open_profile_modal(uid, cx)))
            .child(crate::t!("profile.view_full")),
    );

    // footer_NOlkhE: column, padding 0 16px 12px. The "Message @user" box is
    // a text input (--control-input-height-md 40px, --input-background-default,
    // 1px --input-border-default, --radius-sm).
    let footer = (uid != app.me.id).then(|| {
        div().flex().flex_col().px(z(16.)).pb(z(12.)).child(
            div()
                .id(("profile-message", uid.0))
                .h(z(40.))
                .px(z(12.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .cursor_text()
                .bg(c(tok(idx::INPUT_BACKGROUND_DEFAULT)))
                .border_1()
                .border_color(c(tok(idx::INPUT_BORDER_DEFAULT)))
                .text_size(z(14.))
                .text_color(c(tok(idx::INPUT_PLACEHOLDER_TEXT_DEFAULT)))
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.message_user(uid, cx)))
                .child(crate::t!("profile.message_user", name = i.username)),
        )
    });

    // outer_P1YEjQ.user-profile-popout: --background-surface-high, --radius-sm,
    // --shadow-border + --shadow-high. inner_P1YEjQ: gap 8px, padding-bottom 4px.
    div()
        .relative()
        .w_full()
        .max_h_full()
        .flex()
        .flex_col()
        .gap(z(8.))
        .pb(z(4.))
        .rounded(z(8.))
        .bg(c(bg))
        .shadow_lg()
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .overflow_hidden()
        .child(header)
        .child(body)
        .children(footer)
}

/// Full profile, modal v2 (`root_ZhNJfE`, `layoutContainer_s3DB6Y`): profile
/// card column on the left, tab panel on the right.
fn modal_card(app: &PhoenixApp, uid: Id, guild: Option<Id>, vw: Pixels, cx: &mut Context<PhoenixApp>) -> Div {
    let i = info(app, uid, guild);
    let e = cx.entity();
    let card_bg = tok(idx::BACKGROUND_SURFACE_HIGH); // profile_s3DB6Y background
    // --custom-modal-v2-profile-width: min(max(300px, 45.5vw - 0.91*--space-48), 400px)
    let pw = (f32::from(vw) * 0.455 - 48. * 0.91).clamp(300., 400.);
    // --custom-modal-v2-padding (--space-48 from 1025px, else --space-32)
    let pad = if f32::from(vw) >= 1025. { 48. } else { 32. };
    const GUTTER: f32 = 32.; // --custom-modal-v2-gutter
    const CARD_PAD: f32 = 32.; // --custom-modal-v2-profile-card-padding
    const BANNER: f32 = 140.; // .user-profile-modal-v2 --custom-user-profile-banner-height
    const AV: f32 = 120.; // .user-profile-modal-v2 --custom-user-profile-avatar-size
    const RING: f32 = 7.;

    let e2 = e.clone();
    let buttons = (uid != app.me.id).then(|| {
        // headerInner_NxZlvf: min-height 52px, margin-inline-end 16px;
        // headerButtons_NxZlvf: gap 8px, height --custom-button-button-md-height, padding-top 16px
        div()
            .flex()
            .justify_end()
            .min_h(z(52.))
            .mr(z(16.))
            .child(div().flex().items_center().gap(z(8.)).pt(z(16.)).child(
                mana_button(("profile-modal-message", uid.0), crate::t!("profile.message"), true, false)
                    .on_click(move |_, _, cx| e2.update(cx, |a, cx| a.message_user(uid, cx))),
            ))
    });
    let profile = div()
        .id("profile-modal-card")
        .relative()
        .w(z(pw))
        .flex_none()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .bg(c(card_bg))
        .rounded_t(z(16.)) // --radius-lg --radius-lg 0 0
        .border_1()
        .border_color(c(tok(idx::BORDER_MUTED))) // outline 1px --border-muted
        .shadow_md() // --shadow-medium
        .child(banner(&i, BANNER, 16.))
        // avatar_WRLIGC modal-v2: top banner - avatar/2, start card padding
        .child(div().absolute().left(z(CARD_PAD - RING)).top(z(BANNER - AV / 2. - RING)).child(avatar_block(&i, AV, RING, card_bg)))
        // container_rmCN9_ modal-v2: top banner - --space-8, start card padding + avatar + --space-20
        .when_some(i.custom.clone(), |d, t| {
            d.child(div().absolute().left(z(CARD_PAD + AV + 20.)).top(z(BANNER - 8.)).child(status_bubble(t)))
        })
        .child(div().min_h(z(AV / 2. + 8.)).children(buttons))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(z(12.))
                .px(z(CARD_PAD))
                .pt(z(8.))
                .pb(z(CARD_PAD))
                .child(name_block(&i, true))
                .when_some(badges(&i), |d, b| d.child(b))
                .children(details(&i, guild, None, false)),
        );

    // Right column: tabs (291103 `tabBar_OpoUmC`: gap 32px, padding 0 16px,
    // 1px --user-profile-border bottom; items 25px tall) then the tab panel
    // (a94a5b `tabPanelScroller_y4ZEM5`: padding-top --space-20, bottom --space-32).
    let mutuals = i.prof.map(|p| p.mutual_guilds).unwrap_or(0);
    let tab = |t: String, sel: bool| {
        div()
            .h(z(25.))
            .when(sel, |d| d.border_b_1().border_color(c(tok(idx::TEXT_STRONG))))
            .text_size(z(14.))
            .line_height(z(18.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(if sel { tok(idx::TEXT_STRONG) } else { tok(idx::TEXT_SUBTLE) }))
            .child(t)
    };
    let mut panel = div().flex().flex_col().gap(z(12.)).pt(z(20.)).pb(z(32.));
    let mut any = false;
    for a in i.acts {
        if let Some(b) = activity_block(a) {
            panel = panel.child(b);
            any = true;
        }
    }
    if !any {
        panel = panel.child(
            div()
                .text_size(z(14.))
                .text_color(c(tok(idx::TEXT_MUTED)))
                .child(crate::t!("profile.idle_activity", name = i.display)),
        );
    }
    let right = div()
        .id("profile-modal-tabs")
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .pl(z(GUTTER)) // tabBarPanel_y4ZEM5 padding-inline-start gutter
        .child(
            div()
                .flex()
                .gap(z(32.))
                .px(z(16.))
                .mt(z(16.))
                .border_b_1()
                .border_color(c(tok(idx::USER_PROFILE_BORDER)))
                .child(tab(crate::t!("profile.activity").to_string(), true))
                .child(tab(crate::t!("profile.mutual_servers", n = mutuals).to_string(), false)),
        )
        .child(div().id("profile-modal-panel").flex_1().min_h_0().overflow_y_scroll().child(panel));

    let e3 = e.clone();
    // modalHeaderButtons_ZhNJfE: top --space-8 (header-button-top), end --space-8
    let close = div()
        .id("profile-modal-close")
        .absolute()
        .top(z(8.))
        .right(z(8.))
        .size(z(32.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
        .hover(|s| s.bg(c(tok(idx::CONTROL_ICON_ONLY_BACKGROUND_HOVER))))
        .on_click(move |_, _, cx| {
            e3.update(cx, |a, cx| {
                a.people.close_overlays();
                cx.notify();
            })
        })
        .child(gpui::svg().path(crate::icons::path("close")).size(z(20.)).text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT))));

    // root/outer: --background-base-lower, 1px --border-normal, --radius-lg.
    // profileContentInner_s3DB6Y: padding-block-start / inline --custom-modal-v2-padding, end 0.
    div()
        .relative()
        .size_full()
        .flex()
        .overflow_hidden()
        .rounded(z(16.))
        .border_1()
        .border_color(c(tok(idx::BORDER_NORMAL)))
        .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
        .shadow_lg()
        .pt(z(pad))
        .px(z(pad))
        .child(profile)
        .child(right)
        .child(close)
}
