# Benchmarks

Measured on 2026-11-23 with `scripts/bench/run.sh`. Every number below can be reproduced
with that script.

## Read this first: what is being compared

- **Discord Phoenix** ran on the **mock backend** (`PHOENIX_MOCK=1`): offline fake data, a
  handful of servers, no network, no real images or avatars. A real account with many guilds,
  thousands of cached members and media will use more memory than this. These numbers are a
  floor, not a promise.
- **Official Discord** (desktop 1.0.170, Electron) was **already running on a real account**
  and was measured read-only from `/proc`: summed over all 10 of its processes (main, zygotes,
  GPU, network and audio utilities, renderer). It was not restarted or touched. Its idle CPU
  number is noisy: one sample read 84 % while another tool was attached to it through
  DevTools, and a later quiet sample read 2.2 %. The quiet figure is reported.

So the comparison is a lightly loaded native client against a loaded Electron client. It shows
the order of magnitude, not an exact like-for-like ratio.

## Hardware

| | |
|---|---|
| CPU | Intel Core i7-6700K @ 4.00 GHz (4 cores / 8 threads) |
| GPU | NVIDIA GeForce GTX 1050 Ti (GP107), driver 580 |
| RAM | 32 GB |
| OS | Debian, Linux 7.3.4, KDE Plasma on Wayland |

## Results

| Metric | Discord Phoenix (mock) | Official Discord (real account) |
|---|---|---|
| Binary size | 41.9 MiB (single stripped binary) | Electron app, ~10 processes |
| Cold start to first frame | 1.19 s cold page cache, 0.46–0.52 s warm | not measured (not restarted) |
| Idle RSS after ~30 s | 193–217 MiB | 1129 MiB (sum of all processes) |
| Idle PSS after ~30 s | 141–196 MiB | 805 MiB |
| Idle CPU (mean over 20–30 s) | 0.2–0.45 % of one core | 2.2 % (quiet sample) |
| Process count | 1 | 10 |

Headless numbers (`examples/bench.rs`, offscreen renderer, 1280×800):

| Metric | Value |
|---|---|
| Markdown parse + rich-text layout | 2.4–2.6 µs, 18 allocations per message |
| Mock login → first messages | 0.25 s |
| RSS, main view | 206 MiB |
| Load a 5000-message channel (100 pages of 50, 120 ms mock latency each) | 16 s |
| RSS after loading and scrolling all 5000 messages | 219 MiB (+14 MiB) |
| Full-window frame (refresh + layout + paint, no readback) | 5.0 ms, ~6100 allocations |
| Frame while scrolling, with GPU readback | p50 17.9 ms, p95 22.3 ms |

The headless frame times include copying each frame back from the GPU for capture (about
10 ms). On screen there is no readback, and GPUI only draws when something changes, which is
why idle CPU stays near zero.

## Method

- **Build**: `cargo rustc --release` with the workspace release profile (`lto = "fat"`,
  `codegen-units = 1`, `panic = "abort"`, `strip = true`, `opt-level = 3`).
- **Cold start**: `scripts/bench/run.sh` notes the wall-clock time before `exec`. The app,
  run with `PHOENIX_BENCH=1`, prints the wall-clock time when GPUI reports the first painted
  frame (`Window::on_next_frame`). `PHOENIX_MOCK_AUTOLOGIN=1` skips the login screen, and
  `PHOENIX_BENCH_EXIT_MS` makes the app quit by itself, so nobody has to touch the window.
- **Memory**: `Rss` and `Pss` from `/proc/<pid>/smaps_rollup`, summed over processes.
- **CPU**: the delta of `utime + stime` from `/proc/<pid>/stat` over the sample window,
  divided by wall time, as a percentage of one core.
- **Official Discord**: `scripts/bench/discord_official.sh` finds its processes with
  `pgrep -f 'discord/app-'` and only reads `/proc`.
- **Headless**: `examples/bench.rs` uses a counting global allocator and GPUI's headless
  renderer. `PHOENIX_MOCK_HISTORY=5000` sets the channel length. The example loads older
  pages until it holds 5000 messages, then scrolls through 600 frames of 400 px each.

## Optimisations measured or tried

- **Bounded chat image cache** (`media/lru.rs`). GPUI's default `img()` keeps every decoded
  image and its texture for the life of the window. Chat attachments and embeds now go
  through an LRU with a 128 MiB budget of decoded pixels. Images drawn in the last 256 loads
  are never evicted. Images are requested from Discord's media proxy at 2× their display size,
  and GPUI decodes them on its background executor, never on the UI thread.
- **Embed markdown is parsed once.** Embed descriptions and field values used to be parsed and
  laid out on every frame while visible. They are now cached by text and theme generation
  (512 entries).
- **Message lookups by id use binary search.** Lookups in reply previews (every frame, for
  each visible reply), gateway `MESSAGE_UPDATE`/`DELETE` and duplicate checks on
  `MESSAGE_CREATE` used to scan the whole channel. They now use a binary search over the
  snowflake-ordered history, falling back to a scan only for out-of-order entries. Updates no
  longer clone the previous message.
- **mimalloc**: tried as the global allocator, then dropped. Idle RSS was 199/204 MiB against
  217/193 MiB with glibc, inside the run-to-run noise, and the binary grew by 0.3 MiB.
- **Not done yet**: per-frame allocations, about 6000 for a full window. Most come from
  building GPUI element trees, which is inherent to an immediate-mode UI. No sampling
  profiler is available on this machine (`perf_event_paranoid = 3`, no perf or heaptrack), so
  finer attribution is left for later.

## Reproducing

```sh
scripts/bench/run.sh               # build, headless bench, short real window, official Discord
scripts/bench/run.sh --no-window   # headless only
scripts/bench/discord_official.sh 30
```
