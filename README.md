<p align="center">
  <img src="packaging/icons/discord-phoenix.png" width="128" height="128" alt="Discord Phoenix icon">
</p>

<h1 align="center">Discord Phoenix</h1>

<p align="center">
  A native Discord desktop client written in Rust and rendered on the GPU with <a href="https://gpui.rs">GPUI</a>.<br>
  No Electron, no Chromium, no JavaScript runtime.
</p>

<p align="center">
  <a href="https://github.com/RoxasYTB/discord-phoenix/actions/workflows/ci.yml"><img src="https://github.com/RoxasYTB/discord-phoenix/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/RoxasYTB/discord-phoenix/releases"><img src="https://img.shields.io/github/v/release/RoxasYTB/discord-phoenix?include_prereleases&label=release" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue" alt="License: Apache-2.0 OR MIT">
  <img src="https://img.shields.io/badge/status-beta-orange" alt="Status: beta">
</p>

> [!WARNING]
> **Discord Phoenix is not affiliated with, endorsed by, or sponsored by Discord Inc.**
> "Discord" and the Discord logo are trademarks of Discord Inc. This is an independent, unofficial project.
> Third-party clients are not permitted by Discord's Terms of Service, and using one can lead to your
> account being limited or terminated. You use this software at your own risk.

## Why

The official desktop client is a web application shipped inside Electron. Discord Phoenix reimplements it
as a native application with the goals of:

- **Looking and behaving like the official client**, so there is nothing new to learn.
- **Using far less memory and CPU**, by rendering with the GPU and keeping only what is on screen alive.
- **Reliable audio/video.** Camera and screen share can run at the same time. The client never sends
  H.265/HEVC and only negotiates encodings that decode correctly on Windows, Linux, macOS, iOS and Android
  peers (AV1, H.264 4:2:0 8-bit, VP8), falling back to another codec instead of crashing when an encoder fails.

## Status

Beta: under active development. Expect missing features and bugs.

| Area | State |
|---|---|
| Login (token, email/password, TOTP) | In progress |
| Gateway (compression, resume, reconnect) | In progress |
| Servers, channels, DMs, messages, replies, reactions | In progress |
| Markdown, embeds, attachments | In progress |
| Member list, presence | In progress |
| Voice (Opus) | In progress |
| Camera and screen share | In progress |
| End-to-end encryption for calls (DAVE) | In progress |
| Settings, themes, notifications | Planned |

## Benchmarks

Measurements against the official desktop client will be published here, with the method and
hardware used, once the client can log in and display real servers. Nothing is claimed until then.

## Installing

Prebuilt beta builds for Linux (AppImage, `.deb`, tarball), Windows and macOS are attached to
[releases](https://github.com/RoxasYTB/discord-phoenix/releases). macOS builds are not notarized:
right-click the app and choose **Open** the first time.

## Building from source

Requires a recent stable Rust toolchain (`rustup`).

```sh
git clone https://github.com/RoxasYTB/discord-phoenix
cd discord-phoenix
cargo run -p phoenix-app --release
```

**Linux** also needs these development packages (Debian/Ubuntu names):

```sh
sudo apt install pkg-config libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev libxcb1-dev libfontconfig-dev libssl-dev libasound2-dev
```

**macOS** needs the Xcode command line tools. **Windows** needs the MSVC build tools.

## Project layout

| Crate | Purpose |
|---|---|
| [`phoenix-api`](crates/phoenix-api) | Data models, REST client with rate limiting, gateway connection, state cache |
| [`phoenix-voice`](crates/phoenix-voice) | Voice gateway, UDP/RTP transport, encryption, codec negotiation, DAVE |
| [`phoenix-app`](crates/phoenix-app) | The GPUI desktop application |

## Privacy and security

Your token is only ever sent to Discord. The client has no telemetry and no analytics.
See [SECURITY.md](SECURITY.md) for how credentials are handled and how to report a vulnerability.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) first. In short: everything is
written from scratch; do not copy Discord's proprietary code or assets into this repository.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
