# Voice call view (2026 vanilla client)

Reference dump: `dumps/vanilla-call-view.json` (live vanilla client, focused Go Live + participant strip, chat closed).
Code: `crates/phoenix-app/src/ui/call_view.rs`. Offscreen render: `cargo run -p phoenix-app --example call_view -- out.png [chat]` (mock voice state, never joins voice).

| element | CSS source (file + selector prefix) | property | CSS value | our code | status |
|---|---|---|---|---|---|
| call container | callContainer_hbKzX7 | background / border-top | --black / 1px --app-frame-border | call_view.rs `render` | ✅ |
| top bar | upperContainer_ (dump #1252) | height / padding-inline | 33px / 24px | `top_bar` | ✅ |
| top bar title | title_ h1 body-md/medium (#1255) | font | 16px / 500 / 20px, --text-strong | `top_bar` | ✅ |
| channel icon | channelIcon_ svg (#1257) | size / colour | 20px, --channel-icon | `top_bar` | ✅ |
| chat toggle | chatIcon_Fy2P0m (#1288) | size | 24px, toolbar gap 20 | `top_bar` | ✅ |
| tile | tile_p87Rny (#1405) | radius / overflow | --radius-sm 8px / hidden | `render_tile` | ✅ |
| tile gap | tileSizer_ (#1403, #1448) | spacing | 8px | `grid`, `render` | ✅ |
| tile background | background_ (#1456) | background | avatar dominant colour | `tile_color` | ✅ |
| avatar | avatarWrapper_ (#1436) | size | 80px circle | `render_tile` | ✅ |
| speaking ring | border_p87Rny (#1447) | box-shadow | inset 2px --status-positive + inset 3px --black | `render_tile` | ✅ |
| name / mute pill | experimentOverlayTitle_7NOr2f (#1504) | bg / radius / padding / gap / font | --control-overlay-secondary-background-default / 8 / 6 8 / 4 / 14/18 | `render_tile` pill | ✅ |
| mute/deaf icon | titleIcon_ (#1505) | size | 16px | `render_tile` | ✅ |
| stream placeholder tile | emptyPreviewContainer_ (#1408) | bg | --background-base-lower | `render_tile` stream | ✅ |
| LIVE pill | liveIndicator_ (#1413) | font / radius / bg | 12px 600 / 4px / --status-danger | `render_tile` | ✅ |
| grid layout (no focus) | - | columns | biggest 16:9 tile | `grid` | ✅ (click focused tile → grid) |
| spotlight focused tile | videoSizer_QKgWuD / focusedVideo_QKgWuD (#1372) | box | 684x387 16:9, 72px below top bar, 8 above strip | `render` focus branch | ✅ |
| participants strip | root_coTPKH / tileSizer_coTPKH (#1402-#1514) | height / tile / gap | 112 / (W-gaps)/n ≈135 / 8 | `render` STRIP_H | ✅ |
| button section | buttonSection_9bvEB0 (#1298) | bg / border / radius / gap / padding | --background-surface-high / 1px --border-muted / 12 / 8 / 4 | `section` | ✅ |
| center button | centerButton_K9Xq13 (#1301) | size / radius | 40x40, 8 (attached 8 0 0 8) | `center_button` | ✅ |
| caret nub | contextMenuNub_.attachedCaret_ (#1306) | size / radius / margin | 20x40 / 0 8 8 0 / 1px | `center_button` | ✅ |
| green state | greenGlow_K9Xq13 (#1322) | bg / colour | --opacity-green-12 / --green-300 | `Look::Green` | ✅ |
| red state | redGlow_K9Xq13 | bg / colour | --opacity-red-12 / --red-400 | `Look::Red` | ✅ |
| section gap | wrapper_ (#1297) | gap | 12px | `bottom_controls` | ✅ |
| disconnect | disconnect_K9Xq13.fullRegionButton_ (#1354) | size / radius / bg | 65x49 / 12 / --control-critical-primary-background-default | `bottom_controls` | ✅ |
| edge controls | edgeControls_ (#1291, #1358) | width / gap / icon | 160 / 24 / 24px | `bottom_controls` | ✅ |
| bottom bar | bottomControls_ (#1290) | height / padding / bottom margin | 49 / 20 / 12 | `bottom_controls` | ✅ |
| side chat panel | channelChatWrapper_hbKzX7 + JS store `callChatSidebarWidth ?? 480` (min 450) | bg / width | --background-base-lowest / 480 | `render` CHAT_W=480, no member list, call column min_w_0 | ✅ |
| hover-only name pills (idle opacity 0) | .idle_7NOr2f{opacity:0}, JS IDLE_TIMEOUT:2e3 | opacity | 0 after 2000 ms without mouse move | `idle()` / `on_mouse_move` | ✅ |
| tray / camera icons | icon provider WindowLaunchIcon, FullscreenEnterIcon, VideoSlashIcon (path data matches dump #1362/#1366) | - | - | icons/names.rs `call/*` | ✅ |
| tray responsive collapse | call controls module `bf(width)`: <=650 NO_INVITE, <=550 MINIMAL; bottomControls_rTsZa_ grid minmax(max-content,1fr) auto minmax(max-content,1fr) | invite / 2nd section hidden | 650 / 550 px | `collapse()` / `bottom_controls` | ✅ 1280 px + chat: hang-up fully visible (call_view example) |
| overlay fade | overlayContainer_7NOr2f, actionRow_FowdXv | transition | opacity .2s ease-in-out | `overlay_opacity()` (smoothstep, raf while fading) | ✅ |
| participants strip toggle | participantsButton_1uD6uZ (#1394-#1400) | box / bg / radius / shadow | 60x32, pad 4, --primary-630 (hover 700), r48, --shadow-border+--shadow-high, caret+members 24 | `strip_button` | ✅ |
| mic / camera icon | lottieIcon_69Onuk (#1303, #1312) | toggle animation | Lottie animation on toggle, static at rest | static end-frame icon | ⛔ rest state matches; animation needs a Lottie player. Checked the client's animations in the local cache (Lottie_HoverOnOff_Camera, Lottie_HoverOnOff_Microphone): 4–5 precomp layers, masks, 3 trim paths, 10–17 animated properties each. A subset renderer would need precomps, masks, trim paths (path length) and bezier easing, plus runtime extraction from the cache (the JSON can't be committed). That is not cheap for two icons |
