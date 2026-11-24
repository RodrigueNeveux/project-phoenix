# Performance: start-up and message loading

Goal: instant start and instant channel switches. The UI is drawn from the
disk cache before the network answers, and message pages are served from
memory or disk.

## How to measure

`PHX_TIMINGS=1` makes the app and the examples print `PHX_TIMING t=<ms since
process start> <event> [<duration>ms]` on stderr:

| event | what it times |
|---|---|
| `main_window_opened`, `first_frame` | app start → window built / first frame (app binary only) |
| `connect_start` | token loaded from the keyring, connect begins |
| `rest_get_me` | token check (GET /users/@me), from `connect_start` |
| `gateway_ready` | `connect_start` → READY parsed |
| `ready_built` | `connect_start` → READY converted to UI data |
| `snapshot_loaded` / `snapshot_applied` | READY snapshot read from disk / drawn |
| `ready_applied` | live READY applied to the UI |
| `rest_messages_latest` / `rest_messages_older` | GET /channels/{id}/messages |
| `messages_cache_fresh` / `messages_cache_stale` | page served from the message cache |
| `channel_messages_ready[load\|memory]` | channel opened → messages in app state |
| `channel_switch_frame` | channel opened → first frame rendered with them |
| `media_disk` / `media_net` | avatar / emoji / attachment GET, disk cache hit or network |

Real account, read-only, offscreen (no window, no acks):

```
PHX_TIMINGS=1 PHX_SCALE=1 PHX_W=1280 PHX_H=800 PHX_THEME=dark PHX_AUDIO=00 PHX_WAIT=12 \
  int-bin/real_shots out.png 2>&1 | grep PHX_TIMING
```

Without `PHX_CHANNEL` and with no CDP, it opens the default channel of the
first guild. Some of the time before `connect_start` is the example's own
CDP probes, not the app.

## Baseline (fbb7a46, 2026-11-24, before any cache)

Two cold runs, warm media disk cache.

| step | run 1 | run 2 |
|---|---|---|
| process start → `connect_start` (keyring + example setup) | 2803 ms | 2355 ms |
| `rest_get_me` | 227 ms | 209 ms |
| connect → READY | 1614 ms | 1348 ms |
| READY → UI data | 27 ms | 31 ms |
| process start → READY in UI | ~4440 ms | ~3735 ms |
| GET latest messages | 212 ms | 252 ms |
| channel opened → messages in state | 756 ms | 553 ms |
| channel opened → frame | 756 ms | 553 ms |
| avatars/emoji (104 loads) | all disk hits, < 0.2 ms each | same |

Reading: the user waits ~1.6 s on the gateway and ~0.25 s on the token check
before anything shows, then another ~0.5–0.75 s for the first channel. Media
is not the bottleneck (disk cache is hot; decoding already runs in GPUI's
background image loader).

## Changes and their effect

See the table at the bottom; each row is one commit, measured the same way.

| commit | change | process start → guilds drawn | channel opened → frame |
|---|---|---|---|
| fbb7a46 | baseline | ~3.7–4.4 s | 553–756 ms |
| 510d303 + c0dbb86 | READY snapshot on disk, message page cache (memory LRU 64 + disk), hover prefetch, warm 6 recent channels after READY; token check during the handshake | ~2.1–2.3 s (snapshot drawn ~270 ms after the app starts connecting) | 91–97 ms from the disk page, 8–9 ms from memory |

### After the cache (a10c2a5, three runs)

| step | run 1 | run 2 | run 3 |
|---|---|---|---|
| `connect_start` → `snapshot_loaded` (read + parse READY off-thread) | 181 ms | 202 ms | 186 ms |
| `connect_start` → `snapshot_applied` (guilds, channels, DMs on screen) | 305 ms | 303 ms | 293 ms |
| cached channel → messages in state | 91 ms | 97 ms | 95 ms |
| connect → live READY (network; unchanged) | 3103 ms | 1198 ms | 1234 ms |
| live READY applied, selection kept, page from memory → frame | 9 ms | 8.5 ms | 9 ms |
| `rest_get_me` (now overlaps the handshake) | 207 ms | 198 ms | 199 ms |

The ~1.8 s before `connect_start` in real_shots is the example's CDP probes
(the user's Discord had no CDP port during these runs), not the app.

### Round 2 (649899d, two runs)

Pre-READY history over REST, scroll kept on refresh, unread-mention warm-up
(`warm_channels 7` = 8 max mentions + 6 recent, deduped), snapshot parsed on a
thread at backend creation.

| step | run 1 | run 2 |
|---|---|---|
| snapshot read (disk) | 3.4 ms | 6.8 ms |
| snapshot JSON parse | 148 ms | 198 ms |
| snapshot → UI data | 54 ms | 65 ms |
| `connect_start` → `snapshot_applied` | **112 ms** (was ~300) | **116 ms** |
| cached channel → frame (cold UI) | 150 ms | 167 ms |
| after live READY, page from memory → frame | 13.7 ms | 11.3 ms |

The preload finishes before the window in real_shots; in the app the parse
(~150–200 ms) overlaps window creation instead of following it.

## What's next

- The snapshot cost is now the JSON parse (150–200 ms). A compact binary
  snapshot needs serde on the UI model types (`model.rs`, `SharedString`,
  `ImageRef`) and on the nav events: not done.
- Messages in state take ~95 ms the first time (markdown/rich text of 50
  messages on the UI thread) and ~9 ms when warm: build `RenderedMessage`s
  off-thread.
- Gateway RESUME across restarts: deferred. A resume replays only events
  after the saved seq, but the UI would rebuild from the READY saved at the
  start of the previous run, so everything that changed during that run would
  be lost: read states (channels read become unread again), channel edits,
  presences. It needs a snapshot of the live cache at exit first.
- Prefetch the channels with unread mentions after READY (now only the 6
  most recently viewed channels and hovered rows).
