use std::sync::Arc;

use phoenix_app::{assets::Assets, backend::Backend, discord_backend::DiscordBackend, mock::MockBackend};

fn main() {
    // PHOENIX_MOCK=1 runs on offline fake data (no network, no keyring).
    let mock = std::env::var("PHOENIX_MOCK").is_ok_and(|v| !v.is_empty() && v != "0");
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            phoenix_app::init(cx);
            let backend: Arc<dyn Backend> = if mock {
                Arc::new(MockBackend::new())
            } else {
                let b = DiscordBackend::new();
                cx.set_http_client(b.http_client());
                Arc::new(b)
            };
            phoenix_app::open_main_window(backend, cx);
            cx.activate(true);
        });
}
