use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use gpui::{
    App, AppContext, Bounds, Context, FontWeight, IntoElement, ParentElement, Render, Styled, Window, WindowBounds,
    WindowDecorations, WindowHandle, WindowKind, WindowOptions, div, img, px, rgb, size,
};

pub const SPINNER_ASSET: &str = "splash/videos/connecting.webp";
pub const SIZE: f32 = 300.;
const BACKGROUND: u32 = 0x282b30;
const STATUS: u32 = 0xffffff;

fn install_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut versioned = |root: PathBuf| {
        let Ok(entries) = std::fs::read_dir(&root) else { return };
        let mut v: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("app-")))
            .collect();
        v.sort();
        out.extend(v.into_iter().rev().map(|p| p.join("resources")));
    };
    if let Some(h) = crate::paths::home() {
        for flavor in ["discord", "discordcanary", "discordptb"] {
            versioned(h.join(".config").join(flavor));
            versioned(h.join(".var/app/com.discordapp.Discord/config").join(flavor));
        }
        out.push(h.join(".local/share/flatpak/app/com.discordapp.Discord/current/active/files/discord/resources"));
        out.push(h.join("Applications/Discord.app/Contents/Resources"));
    }
    if let Some(local) = crate::paths::env_dir("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        for flavor in ["Discord", "DiscordCanary", "DiscordPTB"] {
            let root = local.join(flavor);
            let Ok(entries) = std::fs::read_dir(&root) else { continue };
            let mut v: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("app-")))
                .collect();
            v.sort();
            out.extend(v.into_iter().rev().map(|p| p.join("resources")));
        }
    }
    for p in [
        "/opt/discord/resources",
        "/usr/share/discord/resources",
        "/usr/lib/discord/resources",
        "/usr/lib64/discord/resources",
        "/var/lib/flatpak/app/com.discordapp.Discord/current/active/files/discord/resources",
        "/Applications/Discord.app/Contents/Resources",
    ] {
        out.push(PathBuf::from(p));
    }
    out
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

pub fn asar_header(file: &mut std::fs::File) -> Option<(serde_json::Value, u64)> {
    let mut head = [0u8; 16];
    file.read_exact(&mut head).ok()?;
    let header_size = u32_at(&head, 4)? as u64;
    let json_len = u32_at(&head, 12)? as usize;
    let mut json = vec![0u8; json_len];
    file.read_exact(&mut json).ok()?;
    Some((serde_json::from_slice(&json).ok()?, 8 + header_size))
}

pub fn asar_read(archive: &Path, inner: &str) -> Option<Vec<u8>> {
    let mut file = std::fs::File::open(archive).ok()?;
    let (header, base) = asar_header(&mut file)?;
    let mut node = &header;
    for part in inner.split('/') {
        node = node.get("files")?.get(part)?;
    }
    let offset: u64 = node.get("offset")?.as_str()?.parse().ok()?;
    let len = node.get("size")?.as_u64()? as usize;
    file.seek(SeekFrom::Start(base + offset)).ok()?;
    let mut out = vec![0u8; len];
    file.read_exact(&mut out).ok()?;
    Some(out)
}

pub fn client_file(inner: &str) -> Option<Vec<u8>> {
    install_roots().into_iter().map(|r| r.join("app.asar")).filter(|p| p.is_file()).find_map(|p| asar_read(&p, inner))
}

static SPINNER: OnceLock<Option<Arc<gpui::Image>>> = OnceLock::new();

pub fn spinner() -> Option<Arc<gpui::Image>> {
    SPINNER
        .get_or_init(|| client_file(SPINNER_ASSET).map(|b| Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Webp, b))))
        .clone()
}

pub fn preload() {
    std::thread::Builder::new()
        .name("splash-assets".into())
        .spawn(|| {
            let _ = spinner();
        })
        .ok();
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    CheckingForUpdates,
    Starting,
}

impl Status {
    pub fn text(self) -> &'static str {
        match self {
            Status::CheckingForUpdates => "Checking for updates\u{2026}",
            Status::Starting => "Starting\u{2026}",
        }
    }
}

pub struct Splash {
    pub status: Status,
}

impl Render for Splash {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let logo = match spinner() {
            Some(image) => img(image).size(px(200.)).into_any_element(),
            None => div().size(px(200.)).into_any_element(),
        };
        div()
            .size_full()
            .bg(rgb(BACKGROUND))
            .flex()
            .justify_center()
            .items_center()
            .pb(px(10.))
            .font_family(crate::fonts::ui_family())
            .child(
                div().flex().flex_col().items_center().child(logo).child(
                    div()
                        .relative()
                        .top(px(-30.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(16.))
                                .line_height(px(19.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(STATUS))
                                .child(self.status.text()),
                        )
                        .child(div().mt(px(10.)).h(px(8.))),
                ),
            )
    }
}

pub fn window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(SIZE), px(SIZE)), cx))),
        titlebar: None,
        focus: false,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some(crate::APP_ID.into()),
        window_background: gpui::WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
}

pub fn open(cx: &mut App) -> Option<WindowHandle<Splash>> {
    cx.open_window(window_options(cx), |_, cx| cx.new(|_| Splash { status: Status::CheckingForUpdates })).ok()
}

pub fn set_status(handle: WindowHandle<Splash>, status: Status, cx: &mut App) {
    let _ = handle.update(cx, |s, _, cx| {
        s.status = status;
        cx.notify();
    });
}

pub fn close(handle: WindowHandle<Splash>, cx: &mut App) {
    let _ = handle.update(cx, |_, window, _| window.remove_window());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pickle(files: &serde_json::Value, body: &[u8]) -> Vec<u8> {
        let json = serde_json::to_vec(files).unwrap();
        let padded = json.len().div_ceil(4) * 4;
        let mut out = Vec::new();
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&((padded + 8) as u32).to_le_bytes());
        out.extend_from_slice(&((padded + 4) as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&json);
        out.resize(16 + padded, 0);
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn reads_a_file_out_of_an_asar_archive() {
        let header = serde_json::json!({"files": {
            "a.txt": {"size": 3, "offset": "0"},
            "splash": {"files": {"videos": {"files": {"connecting.webp": {"size": 4, "offset": "3"}}}}}
        }});
        let bytes = pickle(&header, b"abcWEBP");
        let dir = std::env::temp_dir().join(format!("phx-asar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("app.asar");
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(asar_read(&path, "a.txt").as_deref(), Some(&b"abc"[..]));
        assert_eq!(asar_read(&path, SPINNER_ASSET).as_deref(), Some(&b"WEBP"[..]));
        assert_eq!(asar_read(&path, "missing"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_texts_match_the_client_splash() {
        assert_eq!(Status::CheckingForUpdates.text(), "Checking for updates…");
        assert_eq!(Status::Starting.text(), "Starting…");
    }
}
