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
