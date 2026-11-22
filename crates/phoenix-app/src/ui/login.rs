//! Login screen styled after Discord's: card with email/password (or token)
//! on the left, QR code panel on the right.

use gpui::{
    Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div,
    img, prelude::*, px,
};
use gpui_component::input::{Input, InputState};

use crate::{
    app::PhoenixApp,
    theme::{Palette, c},
};

fn label(text: &'static str, error: Option<&SharedString>) -> impl IntoElement {
    div()
        .mb(px(8.))
        .flex()
        .gap(px(4.))
        .text_size(px(12.))
        .font_weight(FontWeight::BOLD)
        .text_color(c(if error.is_some() { 0xFA777C } else { Palette::INTERACTIVE }))
        .child(text)
        .map(|d| match error {
            Some(e) => d.child(
                div()
                    .font_weight(FontWeight::NORMAL)
                    .italic()
                    .child(SharedString::from(format!("- {e}"))),
            ),
            None => d.child(div().text_color(c(Palette::RED)).child("*")),
        })
}

fn field(state: &Entity<InputState>) -> impl IntoElement {
    div()
        .h(px(40.))
        .px(px(10.))
        .rounded(px(3.))
        .bg(c(Palette::BG_TERTIARY))
        .flex()
        .items_center()
        .text_size(px(16.))
        .text_color(c(Palette::TEXT_NORMAL))
        .child(Input::new(state).appearance(false).bordered(false))
}

pub fn render(app: &PhoenixApp, _window: &mut Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let err = app.login_error.as_ref();
    let form = div()
        .flex_1()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .flex()
                .justify_center()
                .child("Welcome back!"),
        )
        .child(
            div()
                .mt(px(8.))
                .text_size(px(16.))
                .text_color(c(Palette::INTERACTIVE))
                .flex()
                .justify_center()
                .child(if app.login_with_token {
                    "Paste your user token to continue."
                } else {
                    "We're so excited to see you again!"
                }),
        )
        .child(div().h(px(20.)))
        .map(|d| {
            if app.login_with_token {
                d.child(label("TOKEN", err)).child(field(&app.login_token)).child(
                    div()
                        .mt(px(8.))
                        .text_size(px(12.))
                        .text_color(c(Palette::TEXT_MUTED))
                        .child("Your token is stored only in your system keyring and never shared."),
                )
            } else if app.login_mfa_step {
                d.child(label("TWO-FACTOR CODE", err)).child(field(&app.login_mfa)).child(
                    div()
                        .mt(px(8.))
                        .text_size(px(12.))
                        .text_color(c(Palette::TEXT_MUTED))
                        .child("Open your authenticator app and enter the 6-digit code."),
                )
            } else {
                d.child(label("EMAIL OR PHONE NUMBER", err))
                    .child(field(&app.login_email))
                    .child(div().h(px(20.)))
                    .child(label("PASSWORD", err))
                    .child(field(&app.login_password))
                    .child(
                        div()
                            .mt(px(4.))
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c(Palette::TEXT_LINK))
                            .cursor_pointer()
                            .child("Forgot your password?"),
                    )
            }
        })
        .child(
            div()
                .id("login-btn")
                .mt(px(20.))
                .h(px(44.))
                .rounded(px(3.))
                .bg(c(Palette::BLURPLE))
                .hover(|s| s.bg(c(0x4752C4)))
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(gpui::white())
                .on_click(cx.listener(|a, _, w, cx| a.submit_login(w, cx)))
                .child(if app.logging_in { "…" } else { "Log In" }),
        )
        .child(
            div()
                .mt(px(8.))
                .flex()
                .gap(px(4.))
                .text_size(px(14.))
                .child(div().text_color(c(Palette::TEXT_MUTED)).child("Need an account?"))
                .child(div().text_color(c(Palette::TEXT_LINK)).cursor_pointer().child("Register")),
        )
        .child(
            div()
                .id("toggle-token")
                .mt(px(4.))
                .text_size(px(14.))
                .text_color(c(Palette::TEXT_LINK))
                .cursor_pointer()
                .on_click(cx.listener(|a, _, _, cx| {
                    a.login_with_token = !a.login_with_token;
                    a.login_mfa_step = false;
                    a.login_error = None;
                    cx.notify();
                }))
                .child(if app.login_with_token { "Log in with email instead" } else { "Log in with a token" }),
        );

    let qr = div()
        .w(px(240.))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .child(
            div()
                .size(px(176.))
                .rounded(px(4.))
                .bg(gpui::white())
                .p(px(8.))
                .flex()
                .items_center()
                .justify_center()
                .child(qr_pattern()),
        )
        .child(
            div()
                .mt(px(32.))
                .text_size(px(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child("Log in with QR Code"),
        )
        .child(
            div()
                .mt(px(8.))
                .text_size(px(16.))
                .text_color(c(Palette::INTERACTIVE))
                .text_center()
                .child("Scan this with the Discord mobile app to log in instantly."),
        );

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::linear_gradient(
            135.,
            gpui::linear_color_stop(c(0x404EED), 0.),
            gpui::linear_color_stop(c(0x23272A), 1.),
        ))
        .child(
            div()
                .w(px(784.))
                .p(px(32.))
                .rounded(px(5.))
                .bg(c(Palette::BG_PRIMARY))
                .shadow_lg()
                .flex()
                .gap(px(64.))
                .items_center()
                .child(form)
                .child(div().w(px(1.)).h(px(300.)).bg(c(Palette::BG_PRIMARY)))
                .child(qr),
        )
}

/// Decorative fake QR pattern (no real code: QR login needs the remote-auth
/// gateway, which lives in phoenix-api).
fn qr_pattern() -> impl IntoElement {
    const N: usize = 21;
    let mut col = div().flex().flex_col();
    for y in 0..N {
        let mut row = div().flex();
        for x in 0..N {
            let finder = |cx: usize, cy: usize| {
                let (dx, dy) = (x.abs_diff(cx), y.abs_diff(cy));
                let d = dx.max(dy);
                d <= 3 && d != 2
            };
            let in_finder = (x < 7 && y < 7) || (x >= N - 7 && y < 7) || (x < 7 && y >= N - 7);
            let on = if in_finder {
                finder(3, 3) || finder(N - 4, 3) || finder(3, N - 4)
            } else {
                (x * 7 + y * 13 + x * y) % 5 < 2
            };
            row = row.child(div().size(px(7.6)).when(on, |d| d.bg(gpui::black())));
        }
        col = col.child(row);
    }
    div()
        .relative()
        .child(col)
        .child(
            div()
                .absolute()
                .top(px(64.))
                .left(px(64.))
                .size(px(32.))
                .rounded(px(6.))
                .bg(gpui::white())
                .flex()
                .items_center()
                .justify_center()
                .child(img("icon.png").size(px(28.))),
        )
}
