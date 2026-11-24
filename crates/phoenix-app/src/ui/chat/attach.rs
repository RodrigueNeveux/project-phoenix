//! Non-image attachments: generic file card, audio player and video player chrome.
//! Values come from the 2026 client CSS (`fileWrapper_-9z-XO`, `file_-9z-XO`, `hoverButtonGroup_n2rO26`,
//! `wrapperAudio_FJ99sr`, `audioControls_FJ99sr`, `videoControls_FJ99sr`) and the live file card
//! in the vanilla thread-channel dump (node fileWrapper_ 432x73, icon 30x40, name 16/20, size 12/16).

use gpui::{AnyElement, App, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, img, prelude::*, svg};

use crate::{
    media,
    model::Attachment,
    theme::{FONT_MONO, c, generated::{global, idx}, tok, z},
};

/// Media the chat can load: remote https, or an absolute local path (offscreen mock renders).
pub fn loadable(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with('/')
}

fn tk(i: usize) -> gpui::Hsla {
    c(tok(i))
}

/// `1.23 MB` / `12.34 KB` / `512 bytes`, like the client's file size label.
pub fn human_size(n: u64) -> SharedString {
    if n >= 1 << 20 {
        format!("{:.2} MB", n as f64 / (1 << 20) as f64).into()
    } else if n >= 1024 {
        format!("{:.2} KB", n as f64 / 1024.).into()
    } else {
        format!("{n} bytes").into()
    }
}

fn named_icon(name: &str, size: f32, color: gpui::Hsla) -> gpui::Svg {
    svg().path(crate::icons::path(name)).flex_none().size(z(size)).text_color(color)
}

/// Download button that appears on hover at the card's top-right corner
/// (`hoverButtonGroup_ nonMediaMosaicItem_`: --background-base-low, radius 5, inset -8/-8,
/// outline 1px --background-base-lower; `hoverButton_` padding 6, 20px icon,
/// --interactive-text-default → hover --interactive-background-hover / --interactive-text-hover).
fn hover_download(group: SharedString, url: SharedString) -> AnyElement {
    div()
        .absolute()
        .top(z(-8.))
        .right(z(-8.))
        .rounded(z(5.))
        .overflow_hidden()
        .bg(tk(idx::BACKGROUND_BASE_LOW))
        .border_1()
        .border_color(tk(idx::BACKGROUND_BASE_LOWER))
        .opacity(0.)
        .group_hover(group, |s| s.opacity(1.))
        .child(
            div()
                .id(gpui::ElementId::Name(format!("dl-{url}").into()))
                .p(z(6.))
                .flex()
                .cursor_pointer()
                .text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT))
                .hover(|s| s.bg(tk(idx::INTERACTIVE_BACKGROUND_HOVER)).text_color(tk(idx::INTERACTIVE_TEXT_HOVER)))
                .active(|s| s.bg(tk(idx::INTERACTIVE_BACKGROUND_ACTIVE)).text_color(tk(idx::INTERACTIVE_TEXT_ACTIVE)))
                .child(named_icon("download", 20., tk(idx::INTERACTIVE_TEXT_DEFAULT)))
                .on_click(move |_, _, cx| cx.open_url(&url)),
        )
        .into_any_element()
}

/// Generic file card (`fileWrapper_` 432px / `file_`: --background-surface-high, 1px --border-subtle,
/// radius 8, --shadow-low, padding 16, row, centered).
pub fn file_card(a: &Attachment) -> AnyElement {
    let url = a.url.clone();
    let group: SharedString = format!("file-{}", a.url).into();
    let remote = loadable(&url);
    let link = url.clone();
    div()
        .group(group.clone())
        .relative()
        .w(z(432.))
        .max_w_full()
        .child(
            div()
                .w_full()
                .p(z(16.))
                .rounded(z(8.))
                .bg(tk(idx::BACKGROUND_SURFACE_HIGH))
                .border_1()
                .border_color(tk(idx::BORDER_SUBTLE))
                .shadow(crate::ui::inbox::low_shadow())
                .flex()
                .items_center()
                // icon_: 30x40, margin-inline-end 8px
                .child(named_icon("file", 40., tk(idx::TEXT_LINK)).w(z(30.)).h(z(40.)).mr(z(8.)))
                .child(
                    // fileInner_: flex 1, overflow hidden
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(
                            // filenameLinkWrapper_: --text-link, 16px / 1.25; fileNameLink_:hover underline
                            div()
                                .id(gpui::ElementId::Name(format!("fname-{url}").into()))
                                .text_size(z(16.))
                                .line_height(z(20.))
                                .text_color(tk(idx::TEXT_LINK))
                                .truncate()
                                .cursor_pointer()
                                .hover(|s| s.underline())
                                .child(a.filename.clone())
                                .when(remote, |d| d.on_click(move |_, _, cx| cx.open_url(&link))),
                        )
                        // metadata: body-xs/normal (12/16) --text-muted
                        .child(div().text_size(z(12.)).line_height(z(16.)).text_color(tk(idx::TEXT_MUTED)).child(human_size(a.size))),
                ),
        )
        .when(remote, |d| d.child(hover_download(group.clone(), url.clone())))
        .into_any_element()
}

fn control_icon(id: String, name: &str) -> gpui::Stateful<gpui::Div> {
    // controlIcon_: 24x24 + padding 4, --interactive-text-active, hover --white, active translateY(1px)
    div()
        .id(gpui::ElementId::Name(id.into()))
        .flex_none()
        .p(z(4.))
        .cursor_pointer()
        .text_color(tk(idx::INTERACTIVE_TEXT_ACTIVE))
        .child(named_icon(name, 24., tk(idx::INTERACTIVE_TEXT_ACTIVE)))
}

/// Seek track (`mediaBarWrapper_`: 6px, hsl(--primary-300) / .3; `mediaBarProgress_` --brand-500).
fn track(progress: f32) -> gpui::Div {
    div()
        .flex_1()
        .h(z(24.))
        .mx(z(4.))
        .flex()
        .items_center()
        .child(
            div()
                .relative()
                .w_full()
                .h(z(6.))
                .bg(c(tok(idx::PRIMARY_300)).opacity(0.3))
                .child(div().absolute().top_0().left_0().h_full().w(gpui::relative(progress)).bg(tk(idx::BRAND_500))),
        )
}

fn duration_label() -> gpui::Div {
    // durationTimeWrapper_ margin 4; durationTimeDisplay_ --font-code 12px/12px medium
    div()
        .flex_none()
        .m(z(4.))
        .h(z(12.))
        .flex()
        .font_family(FONT_MONO())
        .text_size(z(12.))
        .line_height(z(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::white())
        .child("0:00")
        .child(div().mx(z(2.)).child("/"))
        .child("0:00")
}

/// Audio attachment card (`wrapperAudio_ newMosaicStyle_`: --background-base-lower, 1px --border-subtle,
/// radius 8, padding 16, width 432; `audioMetadata_` file icon 24x40 + name/size; `audioControls_`
/// --opacity-black-28 bg, --radius-sm, margin-top 4).
pub fn audio_card(a: &Attachment) -> AnyElement {
    let url = a.url.clone();
    let open = if a.original.is_empty() { a.url.clone() } else { a.original.clone() };
    let dl = url.clone();
    div()
        .w(z(432.))
        .max_w_full()
        .p(z(16.))
        .rounded(z(8.))
        .bg(tk(idx::BACKGROUND_BASE_LOWER))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .flex()
        .flex_col()
        .justify_between()
        .child(
            div()
                .flex()
                // audioMetadata_:before 24x40 file glyph at 60% opacity
                .child(named_icon("file", 40., tk(idx::TEXT_LINK)).w(z(24.)).h(z(40.)).opacity(0.6))
                .child(
                    // metadataContent_ padding 0 8px; name ellipsis; size --text-muted, opacity .7
                    div()
                        .flex_1()
                        .min_w_0()
                        .px(z(8.))
                        .overflow_hidden()
                        .child(
                            div()
                                .id(gpui::ElementId::Name(format!("aname-{url}").into()))
                                .text_size(z(16.))
                                .line_height(z(20.))
                                .text_color(tk(idx::TEXT_LINK))
                                .truncate()
                                .cursor_pointer()
                                .hover(|s| s.underline())
                                .child(a.filename.clone())
                                .on_click(move |_, _, cx| cx.open_url(&open)),
                        )
                        .child(div().text_size(z(12.)).line_height(z(16.)).text_color(tk(idx::TEXT_MUTED)).opacity(0.7).child(human_size(a.size))),
                )
                .child(
                    // metadataDownload_ 24px, --interactive-text-default → hover --interactive-text-hover
                    div()
                        .id(gpui::ElementId::Name(format!("adl-{url}").into()))
                        .flex_none()
                        .h(z(24.))
                        .cursor_pointer()
                        .child(named_icon("download", 24., tk(idx::INTERACTIVE_TEXT_DEFAULT)))
                        .on_click(move |_, _, cx| cx.open_url(&dl)),
                ),
        )
        .child(
            div()
                .mt(z(4.))
                .w_full()
                .flex()
                .items_center()
                .rounded(z(global::RADIUS_SM))
                .bg(c(tok(idx::OPACITY_BLACK_28)))
                .child(control_icon(format!("aplay-{url}"), "play"))
                .child(duration_label())
                .child(track(0.))
                .child(control_icon(format!("avol-{url}"), "speaker")),
        )
        .into_any_element()
}

/// Video attachment: poster with the centred play button (`playPausePop_`: --opacity-black-60, round,
/// 24px icon + 12px padding) and the controls bar (`videoControls_`: --opacity-black-60, 32px tall)
/// shown on hover. `wrapper_` radius 3, --background-base-lower.
pub fn video_card(a: &Attachment, (w, h): (f32, f32), _cx: &App) -> AnyElement {
    let poster = media::video_poster(&a.url, (w * 2.) as u32, (h * 2.) as u32);
    let original = if a.original.is_empty() { a.url.clone() } else { a.original.clone() };
    let group: SharedString = format!("video-{}", a.url).into();
    div()
        .id(gpui::ElementId::Name(group.clone()))
        .group(group.clone())
        .relative()
        .w(z(w))
        .h(z(h))
        .max_w_full()
        .rounded(z(8.))
        .overflow_hidden()
        .bg(tk(idx::BACKGROUND_BASE_LOWER))
        .cursor_pointer()
        .child(img(poster).size_full().object_fit(gpui::ObjectFit::Cover))
        .child(
            div().absolute().inset_0().flex().items_center().justify_center().child(
                div()
                    .p(z(12.))
                    .rounded_full()
                    .bg(c(tok(idx::OPACITY_BLACK_60)))
                    .child(named_icon("play", 24., gpui::white())),
            ),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(z(32.))
                .flex()
                .items_center()
                .bg(c(tok(idx::OPACITY_BLACK_60)))
                .opacity(0.)
                .group_hover(group, |s| s.opacity(1.))
                .child(control_icon(format!("vplay-{}", a.url), "play"))
                .child(duration_label())
                .child(track(0.))
                .child(control_icon(format!("vvol-{}", a.url), "speaker"))
                .child(control_icon(format!("vfs-{}", a.url), "call/fullscreen")),
        )
        .on_click(move |_, _, cx| cx.open_url(&original))
        .into_any_element()
}

/// Play / open-externally pill over a video embed's thumbnail (`embedVideoActions_` centred;
/// `wrapper_lj7V-K`: --opacity-black-60, radius 24, height 48, padding 12; `iconWrapper_` opacity .6,
/// hover 1, 24px icons; `iconExternalMargins_` margin-inline 2px 4px).
pub fn video_embed_actions(url: SharedString) -> AnyElement {
    let open = url.clone();
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .h(z(48.))
                .p(z(12.))
                .rounded(z(24.))
                .bg(c(tok(idx::OPACITY_BLACK_60)))
                .flex()
                .items_center()
                .child(
                    div()
                        .id(gpui::ElementId::Name(format!("vplay-embed-{url}").into()))
                        .mx(z(1.))
                        .opacity(0.6)
                        .hover(|s| s.opacity(1.))
                        .cursor_pointer()
                        .child(named_icon("play", 24., gpui::white()))
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                )
                .child(
                    div()
                        .id(gpui::ElementId::Name(format!("vext-embed-{open}").into()))
                        .ml(z(2.))
                        .mr(z(4.))
                        .opacity(0.6)
                        .hover(|s| s.opacity(1.))
                        .cursor_pointer()
                        .child(named_icon("call/popout", 24., gpui::white()))
                        .on_click(move |_, _, cx| cx.open_url(&open)),
                ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::human_size;

    #[test]
    fn sizes_match_client_labels() {
        assert_eq!(human_size(512).as_ref(), "512 bytes");
        assert_eq!(human_size(2048).as_ref(), "2.00 KB");
        assert_eq!(human_size(4_213_337).as_ref(), "4.02 MB");
    }
}
