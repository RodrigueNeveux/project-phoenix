//! "Nouveau fil" sidebar: a thread without a starter message (threads popout
//! "Créer", Ctrl+Shift+Alt+N with no focused message). Structure and values
//! from the live client (ref-cache create-thread-sidebar-{frame,header,default}.json):
//! chatLayerWrapper_ 480px panel with a 1px --border-subtle left edge and a
//! 12px top-left radius; header container_ybUK7Z 49px, padding 8 8 8 20;
//! bottom-aligned form: 64px icon disc, "Nom du fil" input, "Fil privé"
//! checkbox; starter-message textarea at the bottom.
//! Submitting posts `POST /channels/{id}/threads`, then the starter message.

use gpui::{AnyElement, ClickEvent, Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, Subscription, Window, div, prelude::*};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    actions::{ActionReply, ActionRequest},
    app::PhoenixApp,
    model::{ChannelKind, Id},
    theme::{c, generated::idx, tok, z},
};

pub struct CreateThread {
    pub channel: Id,
    pub name: Entity<InputState>,
    pub starter: Entity<InputState>,
    pub private: bool,
    _subs: Vec<Subscription>,
}

impl gpui::Global for Slot {}
#[derive(Default)]
struct Slot(Option<CreateThread>);

pub fn is_open(cx: &gpui::App) -> bool {
    cx.try_global::<Slot>().is_some_and(|s| s.0.is_some())
}

fn t(i: usize) -> gpui::Hsla {
    c(tok(i))
}

fn icon(name: &str, size: f32, color: gpui::Hsla) -> gpui::Svg {
    gpui::svg().path(crate::icons::path(name)).size(z(size)).flex_none().text_color(color)
}

impl PhoenixApp {
    /// Opens the sidebar for `channel` (a guild text/announcement channel).
    pub fn open_create_thread(&mut self, channel: Id, window: &mut Window, cx: &mut Context<Self>) {
        if !self.channels.get(&channel).is_some_and(|c| c.guild.is_some() && matches!(c.kind, ChannelKind::Text | ChannelKind::Announcement)) {
            return;
        }
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(crate::t!("thread.name_placeholder")));
        let starter = cx.new(|cx| InputState::new(window, cx).placeholder(crate::t!("thread.starter_placeholder")));
        let mut subs = Vec::new();
        for input in [&name, &starter] {
            subs.push(cx.subscribe_in(input, window, |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { shift: false, .. } = ev {
                    this.submit_create_thread(window, cx);
                }
            }));
        }
        starter.update(cx, |s, cx| s.focus(window, cx));
        self.chat.thread = None;
        self.parity.threads_open = false;
        cx.set_global(Slot(Some(CreateThread { channel, name, starter, private: false, _subs: subs })));
        cx.notify();
    }

    pub fn close_create_thread(&mut self, cx: &mut Context<Self>) {
        cx.set_global(Slot(None));
        cx.notify();
    }

    fn submit_create_thread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = cx.try_global::<Slot>().and_then(|s| s.0.as_ref()) else { return };
        let (channel, private) = (s.channel, s.private);
        let name = s.name.read(cx).value().trim().to_string();
        let starter = s.starter.read(cx).value().trim().to_string();
        // Client: an empty name falls back to the starter message's start.
        let name = if name.is_empty() { starter.chars().take(40).collect() } else { name };
        if name.is_empty() {
            return;
        }
        let announcement = self.channels.get(&channel).is_some_and(|c| c.kind == ChannelKind::Announcement);
        let _ = window;
        self.action(ActionRequest::CreateStandaloneThread { channel, name, private, announcement }, cx, move |a, r, cx| match r {
            Ok(ActionReply::Channel(id)) => {
                a.close_create_thread(cx);
                a.select_channel(id, cx);
                if !starter.is_empty() {
                    let fut = a.backend.send_message(id, starter, None);
                    cx.spawn(async move |this, cx| {
                        if let Ok(m) = fut.await {
                            let _ = this.update(cx, |this, cx| this.append_message(m, cx));
                        }
                    })
                    .detach();
                }
            }
            Ok(_) => {}
            Err(e) => a.parity_toast(e.0, cx),
        });
    }
}

/// The sidebar, when open for the selected channel.
pub fn panel(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let s = cx.try_global::<Slot>()?.0.as_ref()?;
    if app.selected_channel != Some(s.channel) {
        return None;
    }
    let private = s.private;
    let (name, starter) = (s.name.clone(), s.starter.clone());
    let header = div()
        .flex_none()
        .h(z(49.)) // container_ybUK7Z height 49
        .pt(z(8.)) // padding 8px 8px 8px 20px
        .pr(z(8.))
        .pb(z(8.))
        .pl(z(20.))
        .border_b_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .flex()
        .items_center()
        .child(div().size(z(32.)).flex().items_center().justify_center().child(icon("chat/thread", 20., t(idx::ICON_MUTED)))) // iconWrapper_ 32, svg 20
        .child(
            div()
                .flex_1()
                .text_size(z(16.)) // body-md/medium 16/20 --text-strong
                .line_height(z(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t(idx::TEXT_STRONG))
                .child(crate::t!("thread.name_placeholder")),
        )
        .child(
            div()
                .id("create-thread-close")
                .ml(z(10.)) // toolbar_ padding-left 10
                .size(z(32.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(icon("close", 20., t(idx::INTERACTIVE_ICON_DEFAULT)))
                .on_click(cx.listener(|a, _: &ClickEvent, _, cx| a.close_create_thread(cx))),
        );
    let label = |text: SharedString| {
        div()
            .mb(z(8.)) // labelContainer_ margin-bottom 8
            .text_size(z(16.)) // body-md/medium 16/20 --text-strong
            .line_height(z(20.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(t(idx::TEXT_STRONG))
            .child(text)
    };
    let checkbox = div()
        .size(z(20.)) // checkboxIndicator_ 20, radius 4, 1px border
        .flex_none()
        .rounded(z(4.))
        .border_1()
        .border_color(t(if private { idx::CHECKBOX_BORDER_SELECTED_DEFAULT } else { idx::CHECKBOX_BORDER_DEFAULT }))
        .bg(t(if private { idx::CHECKBOX_BACKGROUND_SELECTED_DEFAULT } else { idx::CHECKBOX_BACKGROUND_DEFAULT }))
        .flex()
        .items_center()
        .justify_center()
        .when(private, |d| d.child(icon("check", 14., t(idx::CHECKBOX_ICON_ACTIVE))));
    let form = div()
        .mx(z(20.)) // container_w6h4R2 margin 20
        .mb(z(20.))
        .flex()
        .flex_col()
        .child(
            div()
                .my(z(16.)) // iconWrapper_ 64px disc, margin 16px 0
                .size(z(64.))
                .rounded_full()
                .bg(t(idx::BACKGROUND_MOD_STRONG))
                .flex()
                .items_center()
                .justify_center()
                .child(icon("chat/thread", 32., t(idx::ICON_SUBTLE))),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(z(24.)) // stack_ gap 24
                .child(
                    div().flex().flex_col().child(label(crate::t!("thread.name"))).child(
                        div()
                            .h(z(40.)) // input wrapper md: 40px, radius 8, 1px --input-border-default
                            .rounded(z(8.))
                            .border_1()
                            .border_color(t(idx::INPUT_BORDER_DEFAULT))
                            .bg(t(idx::INPUT_BACKGROUND_DEFAULT))
                            .px(z(12.)) // input_ padding 1px 12px
                            .flex()
                            .items_center()
                            .text_size(z(16.))
                            .child(Input::new(&name).appearance(false).bordered(false)),
                    ),
                )
                .child(
                    div().flex().flex_col().child(label(crate::t!("thread.private"))).child(
                        div()
                            .id("create-thread-private")
                            .flex()
                            .gap(z(12.)) // checkboxOption_ gap 12
                            .cursor_pointer()
                            .child(checkbox)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(z(16.)) // body-md/normal 16/20
                                    .line_height(z(20.))
                                    .text_color(t(idx::TEXT_STRONG))
                                    .child(crate::t!("thread.private_hint")),
                            )
                            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                                if let Some(s) = cx.global_mut::<Slot>().0.as_mut() {
                                    s.private = !s.private;
                                }
                                cx.notify();
                            })),
                    ),
                ),
        );
    let composer = div()
        .flex_none()
        .mx(z(10.)) // channelTextArea_ inset 10px, radius 8, 1px --border-subtle
        .mb(z(10.))
        .min_h(z(58.))
        .rounded(z(8.))
        .border_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .bg(t(idx::CHAT_BACKGROUND_DEFAULT))
        .pl(z(10.)) // inner_ padding-left 10 (sansAttachButton)
        .pr(z(12.))
        .flex()
        .items_center()
        .text_size(z(16.)) // textArea 16/22
        .line_height(z(22.))
        .child(div().flex_1().child(Input::new(&starter).appearance(false).bordered(false)));
    Some(
        div()
            .id("create-thread")
            .w(z(480.)) // chatLayerWrapper_ 480
            .flex_none()
            .h_full()
            .border_l_1()
            .border_t_1()
            .rounded_tl(z(12.)) // container_FrbQJp border-radius 12px 0 0
            .border_color(t(idx::BORDER_SUBTLE))
            .bg(t(idx::BACKGROUND_BASE_LOWER))
            .flex()
            .flex_col()
            .child(header)
            .child(div().id("create-thread-scroll").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().justify_end().child(form))
            .child(composer)
            .into_any_element(),
    )
}
