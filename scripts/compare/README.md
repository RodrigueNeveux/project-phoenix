# Real-data comparison: Discord vs Phoenix

Renders Phoenix offscreen on the user's **real account**, on the screen the user's Discord shows right now, then compares it with a CDP screenshot of Discord.

## Build once

    nice -n 19 cargo rustc -p phoenix-app --example real_shots -- -L /home/yohan/.local/lib/devlinks

## Run

    ~/discord-phoenix-ref/.venv/bin/python scripts/compare/compare.py

This does four things:
1. It reads Discord's viewport, DPR, location and the DOM boxes of the regions through `cdp.mjs eval`.
2. It takes `cdp.mjs shot`.
3. It runs `real_shots` (same guild/channel, window size, theme and density).
4. It writes these files to `/mnt/windata/project-phoenix/compare/<timestamp>/`:
   - `discord.png` and `phoenix.png`
   - `side_by_side.png` (Discord left, Phoenix right)
   - `diff_heatmap.png` (red/yellow means different, with the region boxes drawn on it)
   - `scores.json`: per-region `mismatch_pct` (pixels whose difference is above 24/255) and `mean_diff`, for title bar, guild rail, channel list, header, messages, composer and member list.

To compare existing images, run `compare.py --discord d.png --phoenix p.png --regions boxes.json`. `boxes.json` comes from a previous run.

### `real_shots` on its own

    real_shots out.png

- `PHX_GUILD` and `PHX_CHANNEL`, or `PHX_DM=<channel id>`, choose the location. The default is the location shown in Discord.
- `PHX_W`/`PHX_H`, `PHX_THEME` (dark|light|ash|onyx) and `PHX_COMPACT` set the window size, theme and density. The default reads them from Discord's `<html>` classes.
- `PHX_WAIT` is how long to wait for messages and images, in seconds (default 15).

It is strictly read-only:
- The token is loaded from the keyring and never printed, saved or cleared.
- `discord_backend::NO_ACKS` turns off every ack and read marker.
- A wrapper backend refuses every send, edit, delete, typing, reaction, pin, voice, status, settings, nav and parity call.

## Privacy

The images and `boxes.json` contain private conversations. Keep them on /mnt/windata only. Never commit them, and never attach them to the dashboard. Post only the scores.
