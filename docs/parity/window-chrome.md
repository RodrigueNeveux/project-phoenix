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
