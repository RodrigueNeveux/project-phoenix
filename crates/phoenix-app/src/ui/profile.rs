//! User profile popout (click on a member / author), full profile modal and
//! the overlay host that also shows the status picker.

use crate::theme::{generated::idx, tok, z};
use gpui::{
    AnyElement, Context, Div, FontWeight, IntoElement, ParentElement, Pixels, Point, SharedString, Styled,
    Window, div, img, prelude::*, px,
};
use gpui_component::input::{InputEvent, InputState};

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
        set_modal_tab(0);
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
    out.extend(crate::ui::thread_menu::render(app, cx));
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
        out.push(status_picker::render(app, window, cx));
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

/// Activity card (`card_Bu4Al8` inside `overlay_P1YEjQ`): popout padding 8px,
/// full profile --space-12 with --radius-md; header `headerText_Bu4Al8`
/// (body-xs/medium), body `content_Bu4Al8` (gap 8px / 12px), title
/// heading-xs/semibold 14/18, lines body-xs 12/16.
fn activity_block(a: &Activity, big: bool) -> Option<AnyElement> {
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
    // imagePosition_1AI0dL: 60px (full profile), --radius-sm
    let s = if big { 60. } else { 48. };
    let art: AnyElement = match &a.image {
        Some(u) => img(u.clone()).size(z(s)).rounded(z(8.)).flex_none().into_any_element(),
        None => div().size(z(s)).rounded(z(8.)).flex_none().bg(c(tok(idx::BACKGROUND_MOD_NORMAL))).into_any_element(),
    };
    Some(
        div()
            .flex()
            .flex_col()
            .gap(z(8.)) // card_Bu4Al8 gap
            .p(z(if big { 12. } else { 8. })) // --custom-activity-card-padding
            .rounded(z(if big { 12. } else { 8. })) // --radius-md / overlay --radius-sm
            .bg(c(tok(idx::USER_PROFILE_OVERLAY_BACKGROUND_HOVER))) // card_Bu4Al8 background
            .border_1()
            .border_color(c(tok(idx::BORDER_SUBTLE)))
            .child(
                div()
                    .text_size(z(12.)) // body-xs/medium
                    .line_height(z(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(tok(idx::TEXT_DEFAULT)))
                    .text_ellipsis()
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .gap(z(if big { 12. } else { 8. })) // content_Bu4Al8 gap
                    .items_center()
                    .child(art)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(z(4.)) // details_Bu4Al8 gap
                            .when_some(line1, |d, l| {
                                d.child(
                                    div()
                                        .text_size(z(14.)) // heading-xs/semibold
                                        .line_height(z(18.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(c(tok(idx::TEXT_STRONG)))
                                        .text_ellipsis()
                                        .child(l),
                                )
                            })
                            .children(lines.into_iter().map(|l| {
                                div()
                                    .text_size(z(12.)) // body-xs/normal
                                    .line_height(z(16.))
                                    .text_color(c(tok(idx::TEXT_DEFAULT)))
                                    .text_ellipsis()
                                    .child(l)
                            })),
                    ),
            )
            .into_any_element(),
    )
}

/// Full profile section heading (`header_NcvRRg` h2 body-xs/medium, 12/16).
fn section_heading(t: SharedString) -> Div {
    div()
        .text_size(z(12.))
        .line_height(z(16.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(idx::TEXT_DEFAULT)))
        .child(t)
}

/// Everything both layouts need about one user.
pub(crate) struct Info<'a> {
    /// Per-guild avatar/decoration applied (`PhoenixApp::member_user`).
    pub(crate) user: Option<std::borrow::Cow<'a, crate::model::User>>,
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
    let user_cow = app.member_user(guild, uid);
    let user = user_cow.as_deref();
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
    // No banner and no accent colour: the client paints the avatar's dominant
    // colour (vanilla-dm-profile-panel #1562 `fill_vNEdYY`), see avatar_color.
    let avatar_colour = match user.map(|u| &u.avatar) {
        Some(ImageRef::Url(u)) if prof.is_none_or(|p| p.banner.is_none()) => crate::ui::avatar_color::get(u),
        _ => None,
    };
    let accent = prof.and_then(|p| p.accent).or(avatar_colour).unwrap_or(fallback_accent);
    let mut role_ids: Vec<Id> =
        prof.map(|p| p.roles.clone()).filter(|r| !r.is_empty()).or_else(|| member.map(|m| m.roles.clone())).unwrap_or_default();
    let guild_roles = guild.and_then(|g| app.roles.get(&g));
    if let Some(gr) = guild_roles {
        role_ids.sort_by_key(|r| -gr.iter().find(|x| x.id == *r).map(|x| x.position).unwrap_or(0));
    }
    let roles = role_ids.iter().filter_map(|r| guild_roles?.iter().find(|x| x.id == *r).cloned()).collect();
    Info { user: user_cow, prof, presence, acts, custom, display, username, accent, roles }
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
        .child(avatar_with_status(i.user.as_deref(), size, i.presence, c(ring_color)))
        .when_some(i.prof.and_then(|p| p.decoration.clone()), |d, deco| {
            // Avatar decorations are drawn at 1.2x the avatar, centred.
            let s = size * 1.2;
            d.child(img(deco).absolute().size(z(s)).left(z(ring - (s - size) / 2.)).top(z(ring - (s - size) / 2.)))
        })
}

/// Badges row (`container_IqWmhA`: inline in the username row, gap 2px,
/// margin 1px 0; `badge_IqWmhA` 20px).
fn badges(i: &Info) -> Option<Div> {
    let b = i.prof.map(|p| p.badges.as_slice()).unwrap_or_default();
    (!b.is_empty()).then(|| {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(z(2.))
            .my(z(1.))
            // managed_IqWmhA: padding 1px 3px, --radius-sm; :before --background-mod-subtle,
            // 1px --border-muted (dump vanilla-dm-profile-panel #1580)
            .px(z(3.))
            .py(z(1.))
            .rounded(z(8.))
            .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
            .border_1()
            .border_color(c(tok(idx::BORDER_MUTED)))
            .children({
                // UserProfileBadgeListRows: first 6 + overflowSlot "+N" (ui::profile_bio).
                let (shown, hidden) = crate::ui::profile_bio::split_badges(b, i.user.as_deref().is_some_and(|u| u.bot));
                shown
                    .iter()
                    .map(|(_, icon)| img(icon.clone()).size(z(20.)).flex_none().into_any_element())
                    .chain((hidden > 0).then(|| crate::ui::profile_bio::overflow_slot(hidden).into_any_element()))
                    .collect::<Vec<_>>()
            })
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

/// Name block (866212 `displayNameRow_C2XpHA`, `usernameAndPronounsRow_`):
/// display name heading-lg/semibold 20/24 (modal heading-xl/semibold 24/32),
/// then username, pronouns, server tag and badges (gap 2px 6px).
pub(crate) fn name_block(i: &Info, big: bool, trailing: Option<AnyElement>) -> Div {
    let tag = i.user.as_deref().and_then(|u| u.tag.clone());
    let pronouns = i.prof.map(|p| p.pronouns.clone()).filter(|p| !p.is_empty());
    let bot = i.user.as_deref().is_some_and(|u| u.bot);
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .group(crate::ui::profile_note::NAME_ROW_GROUP)
                .flex()
                .items_center()
                .gap(z(4.)) // displayNameRow gap --space-4
                .child(
                    crate::fonts::styled_name(div(), i.user.as_deref(), true)
                        .text_size(z(if big { 24. } else { 20. }))
                        .line_height(z(if big { 32. } else { 24. }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .when(i.user.as_deref().is_none_or(|u| u.name_style.as_ref().is_none_or(|s| s.2.is_empty())), |d| d.text_color(c(tok(idx::TEXT_STRONG))))
                        .text_ellipsis()
                        .child(i.display.clone()),
                )
                .when(bot, |d| d.child(app_tag()))
                // displayNameTrailing_C2XpHA: the note button (popout / DM panel)
                .children(trailing),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .when(bot, |d| d.mt(z(8.))) // usernameAndPronounsRow_.isBot margin-top
                .gap_x(z(6.))
                .gap_y(z(if big { 4. } else { 2. }))
                .text_size(z(14.)) // body-sm/normal
                .line_height(z(18.))
                .child(div().h(z(24.)).flex().items_center().text_color(c(tok(idx::TEXT_STRONG))).child(i.username.clone()))
                .when_some(pronouns, |d, p| {
                    // dotSpacer_C2XpHA: 4px round, margin-inline 1px
                    d.child(div().size(z(4.)).mx(z(1.)).rounded_full().bg(c(tok(idx::TEXT_DEFAULT))))
                        .child(div().mr(z(2.)).text_ellipsis().text_color(c(tok(idx::TEXT_DEFAULT))).child(p))
                })
                .when_some(tag, |d, (t, badge)| d.child(guild_tag_pill(t, badge)))
                .when_some(badges(i), |d, b| d.child(b)),
        )
}

/// `chipletContainerInner_k8jyN7.chipletContainerInline_`: 24px, padding 0 8px,
/// --radius-sm, --background-mod-normal; text body-xs/normal 12/16, badge
/// 14px with margin-inline-end 2px.
fn guild_tag_pill(t: SharedString, badge: Option<SharedString>) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("guild-tag-{t}")))
        .h(z(24.))
        .px(z(8.))
        .flex()
        .items_center()
        .rounded(z(8.))
        .bg(c(tok(idx::BACKGROUND_MOD_NORMAL)))
        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_STRONG))))
        .text_size(z(12.))
        .line_height(z(16.))
        .text_color(c(tok(idx::TEXT_STRONG)))
        .when_some(badge, |d, b| d.child(img(b).size(z(14.)).mr(z(2.)).flex_none()))
        .child(t)
}

/// Role pills (`roleListContainer_xL50-d` gap 4px, margin-top 2px; `role_xL50-d`
/// min-height 24px, 1px --user-profile-border, --radius-sm, padding 0 8px 0 2px;
/// `roleCircle_7RsUnm` 12px margin 0 4px; `roleName_` body-xs/normal).
fn roles_block(roles: Vec<crate::model::Role>) -> Div {
    div().mt(z(2.)).flex().flex_wrap().gap(z(4.)).children(roles.into_iter().map(|r| {
        div()
            .flex()
            .items_center()
            .min_h(z(24.))
            .pl(z(2.))
            .pr(z(8.))
            .rounded(z(8.))
            .border_1()
            .border_color(c(tok(idx::USER_PROFILE_BORDER)))
            .text_size(z(12.))
            .line_height(z(16.))
            .text_color(c(tok(idx::TEXT_STRONG)))
            .child(div().size(z(12.)).mx(z(4.)).flex_none().rounded_full().bg(role_color(r.color.or(Some(0x99AAB5)))))
            .child(div().max_w(z(200.)).text_ellipsis().child(r.name.clone()))
    }))
}

/// Bio (`section_NcvRRg` gap 8px; `bioText_qjJ3Wv` body-sm 14/18). The popout
/// clamps it to 3 lines (`descriptionClamp_BcddaK`, max-height 55px) with a
/// "View Full Bio" text button (`viewFullBio_` margin-top 8px, body-xs).
fn bio_block(bio: SharedString, clamp: bool) -> Div {
    let long = clamp && crate::ui::profile_bio::bio_is_long(&bio);
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(z(14.))
                .line_height(z(18.))
                .child(crate::ui::profile_bio::bio(&bio, clamp, c(tok(idx::TEXT_DEFAULT)))),
        )
        .when(long, |d| {
            d.child(
                div()
                    .mt(z(8.))
                    .text_size(z(12.))
                    .line_height(z(16.))
                    .text_color(c(tok(idx::TEXT_DEFAULT)))
                    .child(crate::t!("profile.view_full_bio")),
            )
        })
}

/// `mutuals_PXYMZ1` (gap 2px 6px): stacked 16px friend avatars
/// (`avatar_NbMTpy` margin-inline-end -3px) + "N Mutual Friends",
/// `spacer_` dot 4px --interactive-text-default, "N Mutual Servers".
pub(crate) fn mutuals_row(p: &Profile) -> Option<Div> {
    let friends = p.mutual_friends_count;
    let servers = p.mutual_guilds;
    if friends == 0 && servers == 0 {
        return None;
    }
    let both = friends > 0 && servers > 0;
    // body-xs when both sections are shown, body-sm otherwise.
    let (fs, lh) = if both { (12., 16.) } else { (14., 18.) };
    let text = move |t: SharedString| {
        div().text_size(z(fs)).line_height(z(lh)).text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT))).child(t)
    };
    let section = || div().flex().items_center().gap(z(4.)).py(z(4.));
    let stack = div().flex().children(p.mutual_friends.iter().take(3).enumerate().map(|(n, u)| {
        div().when(n < 2.min(p.mutual_friends.len().saturating_sub(1)), |d| d.mr(z(-3.))).child(crate::ui::avatar(Some(u), 16.))
    }));
    Some(
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(z(6.))
            .gap_y(z(2.))
            .when(friends > 0, |d| {
                d.child(section().when(!p.mutual_friends.is_empty(), |s| s.child(stack)).child(text(crate::t!("profile.mutual_friends", n = friends))))
            })
            .when(both, |d| d.child(div().size(z(4.)).rounded_full().bg(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))))
            .when(servers > 0, |d| d.child(section().child(text(crate::t!("profile.mutual_servers", n = servers))))),
    )
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

/// Round banner button (`bannerButton_C4gRrZ`: --custom-button-button-sm-height
/// 32px, 50%, --control-overlay-secondary-background-default, 1px
/// --opacity-white-8, icon 16px --white).
pub(crate) fn banner_button(id: impl Into<gpui::ElementId>, icon: &str) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .size(z(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .bg(c(tok(idx::CONTROL_OVERLAY_SECONDARY_BACKGROUND_DEFAULT)))
        .hover(|s| s.bg(c(tok(idx::CONTROL_OVERLAY_SECONDARY_BACKGROUND_ACTIVE))))
        .border_1()
        .border_color(c(tok(idx::OPACITY_WHITE_8)))
        .child(gpui::svg().path(crate::icons::path(icon)).size(z(16.)).text_color(c(tok(idx::WHITE))))
}

/// Nitro theme backdrop for `outer_P1YEjQ.custom-user-profile-theme`:
/// `linear-gradient(primary, primary --custom-user-profile-banner-height (105px),
/// secondary)` (sRGB interpolation). Two absolute layers so the 105px stop is
/// exact whatever the card height; the content above lays the uniform
/// `inner_P1YEjQ` overlay (--profile-gradient-overlay-color) on top.
pub(crate) fn theme_backdrop((primary, secondary): (u32, u32)) -> [Div; 2] {
    [
        div().absolute().top_0().left_0().right_0().h(z(105.)).bg(c(primary)),
        div().absolute().top(z(105.)).left_0().right_0().bottom_0().bg(gpui::linear_gradient(
            180.,
            gpui::linear_color_stop(c(primary), 0.),
            gpui::linear_color_stop(c(secondary), 1.),
        )),
    ]
}

/// Kick off the avatar dominant-colour extraction for a banner-less user.
pub(crate) fn ensure_avatar_colour(app: &PhoenixApp, uid: Id, cx: &mut gpui::App) {
    let prof = app.people.profiles.get(&uid);
    if prof.is_some_and(|p| p.banner.is_some() || p.accent.is_some()) {
        return;
    }
    if let Some(ImageRef::Url(u)) = app.users.get(&uid).map(|u| &u.avatar) {
        crate::ui::avatar_color::ensure(u, cx);
    }
}

/// Banner buttons (`wrapper_xpwejc`, gap 8; vanilla-profile-popout #2008,
/// vanilla-dm-profile-panel #1551): "Friend" (UserCheckIcon) for a friend,
/// "Add Friend" (UserPlusIcon) when there is no relationship, then "More"
/// (MoreHorizontalIcon) opening the user context menu. "Open in mod view"
/// (ModerationIcon) only exists for moderators and is not reproduced.
pub(crate) fn banner_buttons(app: &PhoenixApp, uid: Id, guild: Option<Id>, end: f32, cx: &mut Context<PhoenixApp>) -> Div {
    use crate::ui::home::data::{NavRequest, RelAction, RelationshipKind};
    let rel = app.nav.relationship(uid).map(|r| r.kind);
    let bot = app.users.get(&uid).is_some_and(|u| u.bot);
    let friend_btn = match rel {
        Some(RelationshipKind::Friend) => Some(banner_button(("profile-friend", uid.0), "user-check")),
        None if !bot => Some(
            banner_button(("profile-add-friend", uid.0), "invite")
                .on_click(cx.listener(move |a, _, _, cx| {
                    a.nav_request(NavRequest::Relationship { user: uid, action: RelAction::Accept }, cx, |_, _, _| {})
                })),
        ),
        _ => None,
    };
    let more = banner_button(("profile-more", uid.0), "chat/more").on_click(cx.listener(move |_, ev: &gpui::ClickEvent, w, cx| {
        crate::ui::user_menu::open(uid, guild, ev.position(), w);
        cx.notify();
    }));
    div().absolute().right(z(end)).top(z(8.)).flex().gap(z(8.)).children(friend_btn).child(more)
}

fn popout_card(
    app: &PhoenixApp,
    uid: Id,
    guild: Option<Id>,
    _note: Option<&gpui::Entity<InputState>>,
    cx: &mut Context<PhoenixApp>,
) -> Div {
    ensure_avatar_colour(app, uid, cx);
    let i = info(app, uid, guild);
    let bg = tok(idx::BACKGROUND_SURFACE_HIGH); // outer_P1YEjQ background
    let e = cx.entity();
    let e_full = e.clone();
    // header_NOlkhE: banner (105) + 35px; avatar_WRLIGC popout at
    // inset-inline-start 16px, top 61px (6px ring around the 80px avatar).
    // Clicking the avatar or the name opens the full profile.
    let header = div()
        .relative()
        .flex_none()
        .min_h(z(105. + 35.))
        .child(banner(&i, 105., 8.))
        .child(
            div()
                .id(("profile-avatar", uid.0))
                .absolute()
                .left(z(10.))
                .top(z(55.))
                .cursor_pointer()
                .on_click(move |_, _, cx| e_full.update(cx, |a, cx| a.open_profile_modal(uid, cx)))
                .child(avatar_block(&i, 80., 6., bg)),
        )
        // wrapper_xpwejc: inset-inline-end 12px, top --space-8, gap 8px
        .children((uid != app.me.id).then(|| banner_buttons(app, uid, guild, 12., cx)))
        // container_rmCN9_ popout: inset-inline-start 105px, top banner - 7px
        .when_some(i.custom.clone(), |d, t| d.child(div().absolute().left(z(105.)).top(z(98.)).child(status_bubble(t))));

    // body_NOlkhE: column, gap 12px, padding 4px 8px 8px 16px (scrollbar gutter)
    let e_name = e.clone();
    let mut body = div()
        .id(("profile-body", uid.0))
        .flex()
        .flex_col()
        .gap(z(12.))
        .pt(z(4.))
        .pl(z(16.))
        .pr(z(8.))
        .pb(z(8.))
        .min_h_0()
        .overflow_y_scroll()
        .child(
            div()
                .id(("profile-name", uid.0))
                .cursor_pointer()
                .on_click(move |_, _, cx| e_name.update(cx, |a, cx| a.open_profile_modal(uid, cx)))
                .child(name_block(&i, false, (uid != app.me.id).then(|| crate::ui::profile_note::note_button(uid, cx)))),
        );
    if uid == app.me.id {
        body = body.child(div().text_size(z(12.)).text_color(c(tok(idx::TEXT_MUTED))).child(status_label(i.presence)));
    }
    body = body.children(crate::ui::profile_note::note_section(i.prof.map(|p| p.note.clone()), uid, cx));
    match i.prof {
        None => {
            body = body.child(div().text_size(z(12.)).text_color(c(tok(idx::TEXT_MUTED))).child(crate::t!("profile.loading")));
        }
        Some(p) => {
            if uid != app.me.id {
                body = body.children(mutuals_row(p));
            }
            if !p.bio.is_empty() {
                body = body.child(bio_block(p.bio.clone(), true));
            }
        }
    }
    // container_M2WrKU (padding-bottom 8px): activity cards
    for a in i.acts {
        if let Some(b) = activity_block(a, false) {
            body = body.child(div().pb(z(8.)).child(b));
        }
    }
    if guild.is_some() && !i.roles.is_empty() {
        body = body.child(roles_block(i.roles.clone()));
    }

    // footer_NOlkhE: column, padding 0 16px 12px. The "Message @user" box is
    // an inline chat input (`inlineContainer_2x8-Qn`: --input-background-default,
    // --input-border-default, --radius-sm; 44px; inner padding-inline-start 15px;
    // placeholder 14/22; 32px emoji button).
    let footer = (uid != app.me.id).then(|| {
        div().flex().flex_col().px(z(16.)).pb(z(12.)).child(
            div()
                .id(("profile-message", uid.0))
                .h(z(44.))
                .pl(z(15.))
                .pr(z(12.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .cursor_text()
                .bg(c(tok(idx::INPUT_BACKGROUND_DEFAULT)))
                .border_1()
                .border_color(c(tok(idx::INPUT_BORDER_DEFAULT)))
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.message_user(uid, cx)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(z(14.))
                        .line_height(z(22.))
                        .text_ellipsis()
                        .text_color(c(tok(idx::INPUT_PLACEHOLDER_TEXT_DEFAULT)))
                        .child(crate::t!("profile.message_user", name = i.display)),
                )
                .child(
                    div()
                        .size(z(32.))
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .child(gpui::svg().path(crate::icons::path("emoji")).size(z(18.)).text_color(c(tok(idx::ICON_SUBTLE)))),
                ),
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

thread_local! {
    /// Selected full-profile tab: 0 Activity, 1 Mutual Friends, 2 Mutual Servers.
    static MODAL_TAB: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// Select a full profile tab (also used by the offscreen screenshots).
pub fn set_modal_tab(t: u8) {
    MODAL_TAB.with(|c| c.set(t));
}

/// Row of the Mutual Friends / Mutual Servers panels (`row_wt-YD5`: padding
/// 4px, margin 1px 0 1px 12px, --radius-xs; 40px icon with margin-inline-end
/// 10px; name 16/20 medium; subline body-xs/medium).
fn mutual_row(id: impl Into<gpui::ElementId>, icon: AnyElement, name: SharedString, sub: Option<SharedString>) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .p(z(4.))
        .ml(z(12.))
        .my(z(1.))
        .rounded(z(4.))
        .cursor_pointer()
        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))))
        .child(div().mr(z(10.)).flex_none().child(icon))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(tok(idx::TEXT_STRONG)))
                        .text_ellipsis()
                        .child(name),
                )
                .when_some(sub, |d, s| {
                    d.child(
                        div()
                            .text_size(z(12.))
                            .line_height(z(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c(tok(idx::TEXT_DEFAULT)))
                            .text_ellipsis()
                            .child(s),
                    )
                }),
        )
}

/// Full profile, modal v2 (`root_ZhNJfE`, `profileContentInner_s3DB6Y`):
/// profile card column on the left, tab panel on the right.
fn modal_card(app: &PhoenixApp, uid: Id, guild: Option<Id>, vw: Pixels, cx: &mut Context<PhoenixApp>) -> Div {
    let i = info(app, uid, guild);
    let e = cx.entity();
    let card_bg = tok(idx::BACKGROUND_SURFACE_HIGH); // profile_s3DB6Y background
    // --custom-modal-v2-profile-width: min(max(300px, 45.5vw - 0.91*--space-48), 400px)
    let pw = (f32::from(vw) * 0.455 - 48. * 0.91).clamp(300., 400.);
    // --custom-modal-v2-padding (--space-48 from 1025px, else --space-32)
    let pad = if f32::from(vw) >= 1025. { 48. } else { 32. };
    const SYSTEM_BAR: f32 = 32.; // --custom-modal-v2-system-bar-padding
    const GUTTER: f32 = 32.; // --custom-modal-v2-gutter
    const CARD_PAD: f32 = 32.; // --custom-modal-v2-profile-card-padding
    const BANNER: f32 = 140.; // .user-profile-modal-v2 --custom-user-profile-banner-height
    const AV: f32 = 120.; // .user-profile-modal-v2 --custom-user-profile-avatar-size
    const RING: f32 = 7.;

    // profileButtons_s3DB6Y: gap 8px; sm buttons (32px): Message + More.
    let e2 = e.clone();
    let buttons = (uid != app.me.id).then(|| {
        div()
            .flex()
            .items_center()
            .gap(z(8.))
            .child(
                mana_button(("profile-modal-message", uid.0), crate::t!("profile.message"), true, false)
                    .on_click(move |_, _, cx| e2.update(cx, |a, cx| a.message_user(uid, cx))),
            )
            .child(
                div()
                    .id(("profile-modal-more", uid.0))
                    .size(z(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(z(8.))
                    .border_1()
                    .border_color(c(tok(idx::CONTROL_SECONDARY_BORDER_DEFAULT)))
                    .bg(c(tok(idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT)))
                    .hover(|s| s.bg(c(tok(idx::CONTROL_SECONDARY_BACKGROUND_HOVER))))
                    .cursor_pointer()
                    .child(gpui::svg().path(crate::icons::path("chat/more")).size(z(16.)).text_color(c(tok(idx::CONTROL_SECONDARY_TEXT_DEFAULT)))),
            )
    });

    // profileBody_s3DB6Y: gap --space-24, padding 4px 24px 32px 32px, margin-top -4px.
    let mut pbody = div()
        .flex()
        .flex_col()
        .gap(z(24.))
        .mt(z(-4.))
        .pt(z(4.))
        .pl(z(CARD_PAD))
        .pr(z(CARD_PAD - 8.))
        .pb(z(CARD_PAD))
        .child(name_block(&i, true, None))
        .children(buttons);
    match i.prof {
        None => {
            pbody = pbody.child(div().text_size(z(12.)).text_color(c(tok(idx::TEXT_MUTED))).child(crate::t!("profile.loading")));
        }
        Some(p) => {
            if !p.bio.is_empty() {
                pbody = pbody.child(bio_block(p.bio.clone(), false));
            }
            // memberSinceWrapper_EhAvF-: gap 8px; memberSince_ gap 4px with a 16px
            // guild icon before the join date; divider_ 4px dot.
            let since = div()
                .flex()
                .items_center()
                .gap(z(8.))
                .text_size(z(14.))
                .line_height(z(18.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .child(format_date(p.created))
                .when_some(p.joined, |d, j| {
                    let gicon = guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)).map(|g| {
                        crate::ui::invite::guild_icon(Some(&g.icon), &g.name, 16.)
                    });
                    d.child(div().size(z(4.)).rounded_full().bg(c(tok(idx::TEXT_DEFAULT))))
                        .child(div().flex().items_center().gap(z(4.)).children(gicon).child(format_date(j)))
                });
            pbody = pbody.child(div().flex().flex_col().gap(z(8.)).child(section_heading(crate::t!("profile.member_since"))).child(since));
        }
    }
    if guild.is_some() && !i.roles.is_empty() {
        // profileRolesSection_s3DB6Y: gap 4px
        pbody = pbody.child(
            div()
                .flex()
                .flex_col()
                .gap(z(4.))
                .child(section_heading(if i.roles.len() == 1 { crate::t!("profile.role") } else { crate::t!("profile.roles") }))
                .child(roles_block(i.roles.clone())),
        );
    }

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
        // profileHeader_s3DB6Y: 220px tall (banner + avatar overhang + 20px)
        .child(div().h(z(220. - BANNER)).flex_none())
        .child(pbody);

    // Right column (`container_y4ZEM5`: padding-top 20px). Tab bar `top_6zZYhF`:
    // 1px --border-subtle bottom; items body-sm/semibold 14/14, padding-bottom
    // 16px, margin-inline-start 16px (first 0), --text-subtle, selected
    // --text-strong with a 2px underline.
    let mutual_f = i.prof.map(|p| p.mutual_friends_count).unwrap_or(0);
    let mutual_s = i.prof.map(|p| p.mutual_guilds).unwrap_or(0);
    let sel = MODAL_TAB.with(|c| c.get());
    let tab = |n: u8, t: SharedString| {
        let on = sel == n;
        div()
            .id(("profile-modal-tab", n as u64))
            .relative()
            .when(n > 0, |d| d.ml(z(16.)))
            .pb(z(16.))
            .mb(z(-1.))
            .cursor_pointer()
            .text_size(z(14.))
            .line_height(z(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c(if on { tok(idx::TEXT_STRONG) } else { tok(idx::TEXT_SUBTLE) }))
            .hover(|s| s.text_color(c(tok(idx::TEXT_STRONG))))
            .on_click(move |_, window, _| {
                set_modal_tab(n);
                window.refresh();
            })
            .child(t)
            .when(on, |d| {
                d.child(div().absolute().left_0().right_0().bottom_0().h(z(2.)).rounded_t(z(2.)).bg(c(tok(idx::TEXT_STRONG))))
            })
    };
    // tabPanelScroller_y4ZEM5: padding 20px 8px 16px 0, gap 12px (24px for activity)
    let mut panel = div().flex().flex_col().pt(z(20.)).pr(z(8.)).pb(z(16.));
    let empty = |t: SharedString| div().ml(z(12.)).text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_MUTED))).child(t);
    match sel {
        1 => {
            panel = panel.gap(z(12.));
            let friends = i.prof.map(|p| p.mutual_friends.as_slice()).unwrap_or_default();
            if friends.is_empty() {
                panel = panel.child(empty(crate::t!("profile.no_mutual_friends")));
            }
            for u in friends {
                let fid = u.id;
                let e3 = e.clone();
                let icon = crate::ui::avatar_with_status(Some(u), 40., app.presence_of(u.id), c(tok(idx::BACKGROUND_BASE_LOWER))).into_any_element();
                panel = panel.child(
                    mutual_row(("mutual-friend", u.id.0), icon, u.display_name.clone(), None).on_click(move |ev, window, cx| {
                        let at = ev.position();
                        e3.update(cx, |a, cx| {
                            a.people.modal = None;
                            a.open_profile(fid, None, at, window, cx);
                        })
                    }),
                );
            }
        }
        2 => {
            panel = panel.gap(z(12.));
            let ids = i.prof.map(|p| p.mutual_guild_ids.as_slice()).unwrap_or_default();
            if ids.is_empty() {
                panel = panel.child(empty(crate::t!("profile.no_mutual_servers")));
            }
            for gid in ids {
                let Some(g) = app.guilds.iter().find(|g| g.id == *gid) else { continue };
                let gid = g.id;
                let e3 = e.clone();
                // icon_wt-YD5 iconSizeMedium: 40px, --radius-md
                let icon = crate::ui::invite::guild_icon(Some(&g.icon), &g.name, 40.);
                panel = panel.child(mutual_row(("mutual-guild", gid.0), icon, g.name.clone(), None).on_click(move |_, _, cx| {
                    e3.update(cx, |a, cx| {
                        a.people.close_overlays();
                        a.select_guild(Some(gid), cx);
                    })
                }));
            }
        }
        _ => {
            panel = panel.gap(z(24.));
            let cards: Vec<AnyElement> = i.acts.iter().filter_map(|a| activity_block(a, true)).collect();
            if cards.is_empty() {
                panel = panel.child(empty(crate::t!("profile.idle_activity", name = i.display)));
            } else {
                panel = panel.child(div().flex().flex_col().gap(z(8.)).children(cards));
            }
        }
    }
    let right = div()
        .id("profile-modal-tabs")
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .pt(z(20.))
        .child(
            div()
                .flex()
                .ml(z(GUTTER))
                .mr(z(16.))
                .mt(z(4.))
                .border_b_1()
                .border_color(c(tok(idx::BORDER_SUBTLE)))
                .child(tab(0, crate::t!("profile.activity")))
                .child(tab(1, crate::t!("profile.mutual_friends", n = mutual_f)))
                .child(tab(2, crate::t!("profile.mutual_servers", n = mutual_s))),
        )
        // tabBarPanel_y4ZEM5: padding-inline-start gutter (scroller margin -12px)
        .child(div().id("profile-modal-panel").flex_1().min_h_0().pl(z(GUTTER - 12.)).overflow_y_scroll().child(panel));

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
    // profileContentInner_s3DB6Y: padding-top --custom-modal-v2-padding +
    // system bar, inline --custom-modal-v2-padding, bottom 0.
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
        .pt(z(pad + SYSTEM_BAR))
        .pl(z(pad))
        .pr(z(32.))
        .child(profile)
        .child(right)
        .child(close)
}
