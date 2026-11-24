//! Real upload check, hard-locked to ONE channel: #général of the
//! "Aperture Sciences" test server (the user allowed test sends there only).
//! Uses the same REST flow as the app (`send_message_with_uploads`): upload
//! slot request, chunked PUT to Cloud Storage, JSON message. Prints the steps
//! and the progress, never the token or the signed URLs.
//!
//! ```sh
//! cargo run -p phoenix-app --example upload_probe -- <file> "<what is tested>" [chunk KiB, default 4096]
//! ```

use std::sync::Arc;

use phoenix_api::{
    ClientProperties, RestClient,
    models::Snowflake,
    rest::{CreateMessage, FileUpload},
};
use phoenix_app::discord_backend::store::{KeyringStore, TokenStore};

/// The only channel this probe may ever post to.
const TEST_GUILD: u64 = 1369382919948210248;
const TEST_CHANNEL: u64 = 1403447573389381652;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(what)) = (args.get(1), args.get(2)) else {
        anyhow::bail!("usage: upload_probe <file> \"<what is tested>\" [chunk KiB]");
    };
    let chunk = args.get(3).and_then(|c| c.parse::<usize>().ok()).map_or(phoenix_api::rest::uploads::UPLOAD_CHUNK, |k| k * 1024);
    anyhow::ensure!(chunk.is_multiple_of(256 * 1024), "chunk must be a multiple of 256 KiB");
    let bytes = std::fs::read(path)?;
    let filename = std::path::Path::new(path).file_name().unwrap().to_string_lossy().into_owned();
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(async move {
        let token = tokio::task::spawn_blocking(|| KeyringStore.load())
            .await?
            .map_err(anyhow::Error::msg)?
            .ok_or_else(|| anyhow::anyhow!("no token in keyring"))?;
        let rest = RestClient::new(token.expose(), ClientProperties::linux_desktop());
        // Guard: the channel must be #général of Aperture Sciences, checked by GET.
        let ch = rest.get_channel(Snowflake(TEST_CHANNEL)).await?;
        let guild = rest.request_json(reqwest::Method::GET, &format!("/guilds/{TEST_GUILD}"), None).await?;
        let gname = guild["name"].as_str().unwrap_or_default();
        let cname = ch.name.clone().unwrap_or_default();
        anyhow::ensure!(
            ch.guild_id == Some(Snowflake(TEST_GUILD)) && gname == "Aperture Sciences" && cname.contains("général"),
            "refusing: channel/guild names do not match the test channel"
        );
        println!("guard ok: {gname} / {cname}");
        let content = format!("[Test Phoenix] envoyé depuis le client Rust (Phoenix) — {what}");
        let file = FileUpload { filename, bytes, content_type: None, description: None, spoiler: false };
        let progress: phoenix_api::rest::uploads::Progress = Arc::new(|a, b| println!("progress {a}/{b}"));
        let mut msg = CreateMessage::text(content);
        let t = std::time::Instant::now();
        let m = rest.send_message_with_uploads_chunked(Snowflake(TEST_CHANNEL), &mut msg, vec![file], Some(progress), chunk).await?;
        println!(
            "sent in {:?}: message with {} attachment(s): {:?}",
            t.elapsed(),
            m.attachments.len(),
            m.attachments.iter().map(|a| (&a.filename, a.size, &a.content_type)).collect::<Vec<_>>()
        );
        anyhow::Ok(())
    })
}
