//! 240px member list, grouped by hoisted role, virtualized.

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement, Styled, div, list,
    prelude::*, px,
};

use crate::{
    app::{MemberRow, PhoenixApp},
    model::Presence,
    theme::{Palette, c, m, role_color},
    ui::avatar_with_status,
};

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    div()
        .w(px(m::MEMBER_LIST_W))
        .flex_none()
        .h_full()
        .bg(c(Palette::BG_SECONDARY))
        .child(list(app.member_list.clone(), move |ix, _w, cx| row(&entity, ix, cx)).size_full())
}

fn row(entity: &Entity<PhoenixApp>, ix: usize, cx: &mut App) -> AnyElement {
    let app = entity.read(cx);
    let Some(g) = app.selected_guild else { return div().into_any_element() };
    match app.member_rows.get(ix) {
        Some(MemberRow::Header(t)) => div()
            .pt(px(24.))
            .pl(px(16.))
            .pr(px(8.))
            .h(px(40.))
            .text_size(px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c(Palette::TEXT_MUTED))
            .text_ellipsis()
            .child(t.clone())
            .into_any_element(),
        Some(MemberRow::Member(i)) => {
            let Some(mem) = app.members.get(&g).and_then(|ms| ms.get(*i)) else {
                return div().into_any_element();
            };
            let user = app.users.get(&mem.user);
            let offline = mem.presence == Presence::Offline;
            div()
                .id(("member", mem.user.0))
                .ml(px(8.))
                .mr(px(8.))
                .h(px(m::MEMBER_ROW_H))
                .px(px(8.))
                .rounded(px(4.))
                .flex()
                .items_center()
                .gap(px(12.))
                .cursor_pointer()
                .when(offline, |d| d.opacity(0.3))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).opacity(1.))
                .child(avatar_with_status(user, 32., mem.presence, c(Palette::BG_SECONDARY)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(role_color(app.member_color(g, mem)))
                                        .text_ellipsis()
                                        .child(user.map(|u| u.display_name.clone()).unwrap_or_default()),
                                )
                                .when(user.is_some_and(|u| u.bot), |d| {
                                    d.child(
                                        div()
                                            .px(px(4.))
                                            .rounded(px(3.))
                                            .bg(c(Palette::BLURPLE))
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(gpui::white())
                                            .child("APP"),
                                    )
                                }),
                        )
                        .when_some(mem.activity.clone().filter(|_| !offline), |d, a| {
                            d.child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(c(Palette::TEXT_MUTED))
                                    .text_ellipsis()
                                    .child(a),
                            )
                        }),
                )
                .into_any_element()
        }
        None => div().into_any_element(),
    }
}
