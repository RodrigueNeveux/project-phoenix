//! Discord Phoenix UI shell (GPUI). Network-agnostic: see [`backend`].

pub mod app;
pub mod assets;
pub mod auth;
pub mod backend;
pub mod commands;
pub mod discord_backend;
pub mod grouping;
pub mod i18n;
pub mod markdown;
pub mod media;
pub mod mock;
pub mod model;
pub mod presence;
pub mod notifications;
pub mod richtext;
pub mod theme;
pub mod ui;
pub mod voice;

use std::sync::Arc;

use gpui::{
    App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
use gpui_component::{Root, theme::{Theme, ThemeMode}};

pub const APP_ID: &str = "discord-phoenix";

fn window_icon() -> Option<Arc<image::RgbaImage>> {
    image::load_from_memory_with_format(assets::APP_ICON_PNG, image::ImageFormat::Png)
        .ok()
        .map(|i| Arc::new(i.to_rgba8()))
}

pub fn window_options(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("Discord Phoenix".into()),
            ..Default::default()
        }),
        app_id: Some(APP_ID.into()),
        icon: window_icon(),
        window_min_size: Some(size(px(940.), px(500.))),
        ..Default::default()
    }
}

/// Shared app setup (also used by offscreen/headless rendering).
pub fn init(cx: &mut App) {
    gpui_component::init(cx);
    Theme::change(ThemeMode::Dark, None, cx);
    app::bind_keys(cx);
}

pub fn open_main_window(backend: Arc<dyn backend::Backend>, cx: &mut App) {
    let opts = window_options(cx);
    cx.open_window(opts, move |window, cx| {
        window.set_window_title("Discord Phoenix");
        let view = cx.new(|cx| app::PhoenixApp::new(backend, window, cx));
        view.read(cx).focus.clone().focus(window, cx);
        cx.new(|cx| Root::new(view, window, cx))
    })
    .expect("failed to open window");
}
