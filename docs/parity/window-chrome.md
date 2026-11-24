# Window chrome (frameless window, app top bar, window buttons)

CSS: ~/discord-phoenix-ref/current-client/css/300559.*.css. Dump: vanilla-guild-text (#13 bar, #46..#52 window buttons).

| element | CSS source | property | CSS value | our code | status |
|---|---|---|---|---|---|
| window | Electron `frame: false` | decorations | frameless | lib.rs window_options `WindowDecorations::Client` | ✅ (needs live check) |
| top bar | `bar_3zSH5d` | height | `--custom-app-top-bar-height` 32px | title_bar.rs render | ✅ |
| top bar | `bar_3zSH5d` | `-webkit-app-region` | drag | window_chrome::drag_region (move on drag, dblclick = maximize) | ✅ |
| bar buttons | `clickable_` / `winButtons_` | `-webkit-app-region` / pointer-events | no-drag / auto | `.occlude()` | ✅ |
| win buttons | `winButtons_3zSH5d` | gap | `--space-4` 4px | win_buttons | ✅ |
| divider | `winButtonsWithDivider_:before` | border / height / margin-inline | 1px `--border-subtle` / 32-12 / 4px | win_buttons | ✅ |
| button | `winButton_3zSH5d` | size | 32×32 | win_buttons btn | ✅ |
| glyph | dump #48/#50/#52 | box | 12×12 | glyph_box | ✅ |
| minimize glyph | dump #48 | shape | 10×1 rect at (1,6) | win-min-g | ✅ |
| maximize glyph | dump #50 | shape | 9×9 rect at 1.5, 1px stroke | win-max-g (10×10 border) | ✅ |
| close glyph | dump #52 | shape | 10×10 cross | assets `icons/window/close.svg` (own drawing) | ✅ |
| color | `winButtons_` | color | `--interactive-text-default` | fg | ✅ |
| min/max hover | `winButtonMinMax_:hover` | bg / color | `--interactive-background-hover` / `--interactive-text-hover` | hover + group_hover | ✅ |
| min/max active | `winButtonMinMax_:active` | bg / color | `--interactive-background-active` / `--interactive-text-active` | active + group_active | ✅ |
| close hover | `winButtonClose_:hover` | bg / color | `--control-critical-primary-background-default` / `-icon-default` | hover + group_hover | ✅ |
| resize edges | Chromium frameless Linux | inner border | 5px, corners 16px, ns/ew/nwse/nesw cursors | window_chrome::resize_handles | ✅ (no CSS; compositor resize) |
| maximized | — | resize strips | none when maximized/tiled/fullscreen | resize_handles | ✅ |

State coverage: default, hover, active (min/max), close hover. No focus/tooltip on window buttons in the client.
| close button | client "Minimize to tray" (default on) | behaviour | hide to tray | title_bar win-close → minimize when tray is up (Wayland cannot unmap); tray.rs SNI Open/Quit | ✅ (needs live check) |

## System tray (`tray.rs`)

Reference: `discord_desktop_core` `systemTray` module (core.asar of app 1.0.171, read 2026-11-24) and the
web client's `SystemTrayManager` (strings `CK()`, states `C$()`).

| Item | Client | Phoenix | Status |
|---|---|---|---|
| Protocol | Electron `Tray` on Linux = StatusNotifierItem + dbusmenu | zbus SNI (`org.kde.StatusNotifierItem-<pid>-1`) + `com.canonical.dbusmenu` | ✅ |
| Left click | `click` → `onTrayClicked` (show the window) | `Activate` / `SecondaryActivate` → activate the window | ✅ / ⚠ KWin only raises a minimised window when the request carries a fresh input serial: the SNI activation token (`ProvideXdgActivationToken`) cannot be handed to GPUI yet |
| Tooltip | `setToolTip(appName)` | `ToolTip` = "Discord Phoenix" | ✅ |
| Menu | `[Open {app}]`, then, connected only, `Rendre muet` / `Mettre en sourdine` (checkboxes) and `Ouvrir les paramètres vocaux et vidéo`, then `Rechercher les mises à jour…` / `Remerciements`, then `Quitter {app}`; a separator before each non-empty group | same order and labels (fr/en from the client's strings), checkmarks follow the panel mute/deafen, Voice settings opens Settings > Voix & vidéo, Remerciements opens `discord.com/acknowledgements` | ✅ / ➖ no "Check for updates" (Phoenix has no updater) |
| Icons | `tray`, `tray-unread`, `tray-connected`, `tray-speaking`, `tray-muted`, `tray-deafened` from `app/images/systemtray/linux`; priority connected → deafened > muted > speaking > connected, else unread, else default | the same PNGs loaded at runtime from the local Discord install (never committed), app icon as fallback; state polled every 500 ms, `NewIcon` / `LayoutUpdated` signals | ✅ / ⚠ unread = any mention or unread DM (client `hasAnyUnread`), server mute/suppress not modelled |
| Close hint balloon | `displayBalloon` (Windows only) | — | ➖ |

Tests run on a separate process only: the user's Phoenix and windows are never closed or minimised.
