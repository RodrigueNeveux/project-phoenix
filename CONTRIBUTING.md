# Contributing to Discord Phoenix

Thanks for helping. Discord Phoenix is an **unofficial** client. It is not affiliated with, endorsed by, or supported by Discord Inc. "Discord" is a trademark of Discord Inc. and is used here only to describe what the client connects to.

## Clean-room rule (mandatory)

We implement against publicly documented behaviour only: the [official developer docs](https://discord.com/developers/docs), community documentation, and observation of network traffic from your own account.

You must **never** contribute:

- code copied, translated or derived from Discord's official clients (desktop, web or mobile JS bundles, decompiled or deobfuscated sources, native modules);
- Discord's proprietary assets: logos, Clyde, sounds, fonts, icons, illustrations, CSS or translation strings;
- anything you obtained under an NDA or by breaching an agreement.

If you have studied Discord's client source closely, describe the behaviour in an issue and let someone else implement it. PRs that break this rule will be closed and the history rewritten if needed.

We also don't accept self-bot features, spam/automation tools, or ways to bypass rate limits, Nitro restrictions or moderation.

## Development

```sh
cargo build
cargo run -p phoenix-app
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Linux needs: `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libvulkan-dev libxcb1-dev libfontconfig-dev libssl-dev libasound2-dev pkg-config`.

Workspace layout:

- `crates/phoenix-api`: REST + gateway client
- `crates/phoenix-voice`: voice/RTC
- `crates/phoenix-app`: GPUI desktop app

## Pull requests

- Keep PRs focused; one topic each.
- CI (fmt, clippy with `-D warnings`, tests on Linux/Windows/macOS) must pass.
- Tests and fixtures must not contain real tokens, user IDs or message content.
- By contributing, you agree your work is dual-licensed under MIT OR Apache-2.0, like the project.

## Releases

Pushing a tag `vX.Y.Z` (or `vX.Y.Z-beta.N`) triggers `.github/workflows/release.yml`. Tags containing `beta`, `alpha` or `rc` are published as pre-releases.
