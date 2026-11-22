# Security Policy

## Reporting a vulnerability

Please **do not open a public issue**. Use [GitHub private vulnerability reporting](https://github.com/RoxasYTB/discord-phoenix/security/advisories/new). Include affected version/commit, platform, and reproduction steps. We aim to acknowledge within 7 days and fix serious issues before disclosure.

Vulnerabilities in Discord's own services should go to Discord's bug bounty programme, not to us.

## Supported versions

Only the latest release (including pre-releases) receives fixes.

## Token handling

A Discord token gives full access to an account. Discord Phoenix handles it as follows, and contributions must keep it that way:

- The token is stored only in the OS credential store (Secret Service/KWallet on Linux, Credential Manager on Windows, Keychain on macOS), never in plain-text config files.
- It is sent only to Discord's official API/gateway hosts over TLS, never to third-party servers, telemetry or update endpoints.
- It must never appear in logs, panic messages, crash reports or `Debug` output; redact it at the type level.
- Logging out removes it from the credential store.

If you find a code path that leaks a token, treat it as a security vulnerability and report it privately.

## Users

Never share your token, logs containing it, or HAR files. Nobody from this project will ask for it. If you think your token leaked, change your Discord password, which invalidates existing tokens.
