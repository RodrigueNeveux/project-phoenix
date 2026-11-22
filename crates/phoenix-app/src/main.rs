use std::sync::Arc;

use phoenix_app::{assets::Assets, mock::MockBackend};

fn main() {
    gpui_platform::application()
        .with_assets(Assets)
        .run(|cx| {
            phoenix_app::init(cx);
            phoenix_app::open_main_window(Arc::new(MockBackend::new()), cx);
            cx.activate(true);
        });
}
