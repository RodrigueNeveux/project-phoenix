# Context menus — parity audit

Reference: vanilla dumps `/mnt/windata/project-phoenix/dumps/vanilla-menu-*.json` (private) and
`menu_M3qk4j` in current-client/css; positioning from ReferencePositionLayer in web.*.js.
Renders: `PHX_OPEN=guildmenu|chanmenu[:n]|catmenu[:n]|usermenu[:n]|msgmenu int-bin/real_shots out.png`.

| menu | dump | items / order / separators | status |
|---|---|---|---|
| server | vanilla-menu-guild, `816581f077f92643.js` | Mark As Read (disabled when read) / Invite / Mute >, Notification Settings > (subtext, clicking opens GuildNotify modal), Hide Muted ☐, Show All ☐ / Server Settings > (any of Manage Guild/Roles/Nicknames/Webhooks/Expressions, Ban, View Audit Log, or owner), Privacy, Per-server Profile / Create Channel, Category, Event / Move To… > / Security Actions, Report Raid, Leave (danger) / Copy ID | ✅ |
| folder | vanilla-menu-folder, `225cc9ca3d5899d3.js` | Mark Folder As Read / Move To… > / Folder Settings, Close All Folders | ✅ |
| text / announcement / forum channel | vanilla-menu-channel, `d4030cd60f4ad310.js` | Mark As Read / Favorites > / Invite, Pin to Top, Copy Link / Mute >, Notification > / Move To… > / Edit, Clone, Create Text, Delete (danger) / Copy ID | ✅ |
| voice / stage channel | `455341c02bc5364b.js` | Mark As Read / Favorites > / Invite to Voice Channel, Pin to Top, Copy Link / Open Chat, Hide Names ☐ / Mute > / Move To… > / Edit, Clone, Create Voice, Delete (danger) / Copy ID | ✅ (Stage now uses vocal layout) |
| notification submenu | vanilla-menu-channel-notify | Use Category Default (subtext) ◉, All, @mentions, Nothing | ✅ (fr label "@mentions" fixed) |
| category | vanilla-menu-category, `f167976c224cba1c.js` | Mark As Read / Collapse ☐ (checkbox), Collapse All (hidden when all categories are collapsed) / Mute >, Notification > / Move To… > / Edit, Delete / Copy ID | ✅ |
| textarea | `14ae1b9e3d94518f.js` | Suggestions >, Show Send Message Button ☐ / spell corrections / Add/Remove from Dictionary, Spellcheck ☐, Languages / Cut, Copy, Paste, Paste as Plain Text | ✅ |
| DM | vanilla-menu-dm | Mark As Read / Pin, Favorites > / Profile, Call, Add Note (subtext), Friend Nickname, Close DM / Apps >, Invite >, Remove Friend, Ignore, Block / Mute @x > / Copy User ID, Copy Channel ID | ✅ |
| group DM | vanilla-menu-group-dm | Mark As Read / Pin, Favorites > / Invites, Edit Group / Mute > / Leave (danger) / Copy ID | ✅ |
| member | vanilla-menu-member-2/-5, menus-a/user-avatar-other (2026-11-27) | Profile, Mention, Send Message, Call, Add/Edit Note, Friend Nickname / Change Nickname, Apps >, Invite >, Remove Friend, Ignore, Block / Roles > (checkboxes) / Mod View, Timeout (non-bots), Kick, Ban / Copy ID | ✅ (wired on member list, chat avatars/usernames, reply authors, and `<@id>` mentions; Timeout hidden on bots) |
| member in voice | vanilla-menu-member, menus-a/voice-sidebar-other (2026-11-27) | Profile, Mention, Send Message, Note, Watch Stream / User Volume slider / Mute ☐, Mute Soundboard ☐, Disable Video ☐, Change Nickname, Apps >, Ignore, Block / Roles >, Move To > / Mod View, Server Mute ☐ (danger), Server Deafen ☐ (danger), Disconnect (danger), Kick/Ban / Copy ID | ✅ (shown for any voice participant from sidebar, call tiles, or member list; Soundboard Mute & Disable Video checkboxes added; Server Mute/Deafen danger-styled) |
| own user | vanilla-menu-member-1, menus-a/user-avatar-own (2026-11-27) | Profile, Mention / Per-server Profile, Apps > / Roles > / Mod View / Copy ID (no note row) | ✅ |
| message | vanilla-menu-message, menus-a/msg-other, menus-a/msg-own-img, menus-a/msg-image (2026-11-27) | 4 quick reactions / Add Reaction, View Reactions / Edit (own), Reply, Forward, Thread (hidden on system messages) / Copy Text, Pin (perm, hidden on system messages), Bookmark, Remind Me > (30m,1h,4h,tomorrow,Monday,custom), Apps >, Mark Unread, Copy Link, Speak / Remove Reactions > (danger), Remove All (danger), Delete (own or Manage Messages), Report (others) / Copy ID | ✅ |
| message: Add Reaction > | vanilla-menu-message-addreaction (current-client/dumps) | 12 most used emoji (20px emoji + :name:) / View More (picker) | ✅ |
| message: View Interaction Info (app messages) | vanilla-menu-message | `ui/chat/panels.rs` + `commands/store.rs` `interaction_info` (shown on application/interaction/bot messages, opens info toast with app name, app ID and message ID) | ✅ |
| image (right-click on an image) | vanilla-menu-image (current-client/dumps) | message menu + / Copy Image, Save Image / Copy Image Link, Open Image Link | ✅ (copy and save image fetch the file: `PhoenixApp::copy_image` / `save_image`) |
| forum post (thread) | vanilla-menu-thread (current-client/dumps, 2026-11-24) | Mark As Read (disabled) / Favorites > / Follow, Close, Lock-Unlock, Edit, Edit Tags >, Copy Link / Mute (disabled), Notification Settings (disabled) / Pin-Unpin, Delete (danger) / Copy Thread ID | ✅ forum post cards (`ui/thread_menu.rs`) + sidebar thread rows (`nav_menus::Target::Thread` wired on `sidebar::thread_row`) |
| custom emoji in a message | vanilla-menu-custom-emoji | the plain message menu, nothing emoji-specific | ✅ (same handler as the message) |
| link in a message | vanilla-menu-link | message menu + / Copy Link, Open Link (ids `message-copy-native-link`, `message-open-native-link`) | ✅ (ui/chat.rs rich: right-click on a text link arms `panels::set_menu_link`; icons chat/link, menu/open-link) |
| panel | menu_M3qk4j | radius 8, 1px --border-subtle, --background-surface-higher, --shadow-high, min 188 / max 320, scroller 8px | ✅ |
| rows | item_M3qk4j | 34 (text) / 36 (icon) / 52 (subtext), padding 8, 14/18 medium, icon 20 + mr 8 | ✅ |
| edge nudging | web.*.js nudgeTopAlignment | 12px window margin, max-height = window − top − 12 (was 16) | ✅ |
| submenu stacking (27/11) | CDP on `channel-context` → `channel-context-channel-notifications` hovered: menu and `submenu_M3qk4j` both 1px `--border-subtle` r8 + `0 12px 24px rgba(0,0,0,.24)`, no outline; the submenu is its own layer above the parent, so the parent's border never crosses it; parent row `focused_M3qk4j` bg `--interactive-background-hover` r4; keyboard focus only moves `focused_` (no focus ring, `:focus-visible` on the menu with outline none); submenu top = row top − 9, left = menu right − 5 | ✅ GPUI paints a div's border after its children, so the panel border is now an overlay drawn before the submenu (context_menu.rs `panel_at`), `SUB_OVERLAP` 5 |
| keyboard | web.*.js list navigators | Up/Down wrap, Home/End, Right/Left submenu, Enter/Space, Esc | ✅ (type-ahead not seen in the client) |
| submenu placement | web.*.js `MenuSubmenuItem` (`Ow` layer: `placement` from the parent context, default `right-start`, `spacing` -4, `viewportPadding` 48, `shiftPadding` 8, `crossAccessFlip` false) | opens on the right of the menu, overlapping it by 4 px; when it does not fit on the right within 48 px of the window edge and the left has more room, it opens on the left (`left-start`), its right edge 4 px inside the menu. Before 27/11 Phoenix only snapped it back into the window, so near the right edge (member list menus) it covered the menu it came from (seen on the real platform under Xvfb) | ✅ `context_menu::sub_flips`, test `a_member_menu_submenu_at_the_right_edge_opens_on_the_left_of_the_menu` |

## Button pass (wt/btn-home, 26/11)

| entry | behaviour now | source | status |
|---|---|---|---|
| server: Server Insights | opens Server Settings on Analytics, shown with the same rule as the settings page (community + Manage Server) | chunk `6c30816cda95e0b0.js` id `insights` → `en.A.open(id, BEX.ANALYTICS)` | ✅ |
| server: Report Raid | shown with Ban, Kick, Timeout or Manage Server on a community server; opens the module 935074 modal (title `uYPGsS`, subtitle `Hg8Ee7` with the help link to article 10989121220631, checkbox group of the five raid types in the client order, gap 16 / padding 8 0 4 from `group_Si_H15`, Cancel / Submit); Submit sends `POST /guilds/{id}/auto-moderation/report-raid` through the guard, which refuses it with the blocked-write toast | `6c30816cda95e0b0.js` (`report-raid`, `$.ic` module 974103), CDN chunk `5ef1ef49ddc769e5.js` (935074), `aed76361ef0ef614.js` (859706 `B`), CSS `3942.05b22d8cf5539c36.css` | ✅ |
| user / DM / message: Apps > | disabled "No Commands Available" row (`YSNlV2`), as the client shows when the app list is empty | `b1ab6c34147db858.js` `menu-commands-empty` | ✅ (Phoenix does not list user-installed app commands yet) |

## Message Details (wt/polish-mix, 26/11)

| item | client | Phoenix | status |
|---|---|---|---|
| « Détails du message » | chunk `15299af603b47329.js` `tZ`: with Developer Mode, clipboard support and a right-click on an emoji (with an id) or a sticker, the « Copier l'identifiant du message » row becomes the submenu `IqqJNI` (CircleInformationIcon): Copy Message ID (`zBoHlf`), Copy Emoji / Sticker ID (`Ap2oVy` / `SJ3249`), and with an image Copy / Open Emoji / Sticker Image Link (`cIoudn` / `B1ubHx`, `gDAM2n` / `qAEi+C`, LinkIcon / BrowserIcon); the link group `tJ` is hidden while the submenu shows | `panels::details_items`, target set on right mouse down by the custom emoji cell (`message_ext`) and the sticker attachment (`chat::attachment`), image = CDN emoji `size=44` / sticker URL | ✅ ⚠ the trailing LinkExternalSmall indicator on « Ouvrir » is not drawn (menu rows have no trailing icon) |

## Audit 27/11 (wt/menus-a): message, user, voice participant

Source: live client over CDP on Aperture #général (dumps `menus-a/*.json` in the private dumps folder, read 2026-11-27 ~17h, French locale, Developer Mode on, user is admin), plus the client chunks fetched from the CDN: guild user menu `f405ca3a57e52266.js` (365689), thread user menu `a97a9554a7091bcb.js`, admin items `c183e2b4cdf42732.js` (291247), move `e7bedb588292d24a.js` (252726), local voice items `22322313ab7c958a.js` (198229), volume `99362d2846a238bf.js` (972432), roles `733502a1b0cc89ed.js` (180798), kick/ban rules `1c56c820a070db2e.js` (351001), timeout rule web.js 838111.

| menu | client (27/11) | Phoenix | status |
|---|---|---|---|
| own message (image + reactions, admin) | 4 quick reactions / Add Reaction >, View Reactions / Edit, Reply, Forward, Create Thread / Pin, Bookmark, Create Reminder >, Apps >, Mark Unread, Copy Message Link / Remove Reactions > (red), Remove All Reactions (red), Delete (red) / Copy Message ID; no Copy Text or Speak without text, no Report | same builder (`chat/panels.rs` `message_menu`) | ✅ |
| other's message | … / Reply, Forward, Thread / Copy Text, Pin, Bookmark, Reminder >, Apps >, Mark Unread, Copy Link, Speak / Delete (Manage Messages), Report / Copy ID | same | ✅ |
| image in a message | message menu + / Copy Image, Save Image / Copy Image Link, Open Image Link | same | ✅ |
| reminder submenu | in 30 m, in 1 h, in 4 h, Tomorrow at 9:00, Monday at 9:00, Custom... | same | ✅ |
| other user (chat avatar) | Profile, Mention, Send Message, Call, Add Note (subtext), Add Friend Nickname / Change Nickname, Apps >, Invite to Server >, Remove Friend, Ignore, Block (red) / Roles > / Mod View, Timeout x, Kick x, Ban x (red, `user.username`) / Copy User ID | same | ✅ |
| own user | Profile, Mention / Per-server Profile, Apps > / Roles > / Mod View / Copy User ID | same | ✅ |
| Roles submenu | role dot (12 px circle in a 20 px box, mr 8, `roleDotAccessory_M3qk4j`) + checkbox; roles I cannot manage (managed or not below my highest role, unless owner) are hidden, or shown disabled when the member has them | was every role, enabled, no dot | ✅ fixed (`can_manage_role`, `MenuItem::with_dot` / `as_disabled`) |
| Timeout / Kick / Ban | need `canManageUser` (role hierarchy, never the owner); Timeout also not for bots, the owner or administrators; Ban not for bots; Kick allowed on bots | permission only | ✅ fixed (`can_manage_user`, `is_admin`) |
| voice participant (sidebar, other, bot) | Profile, Mention, Send Message, Add Note / User Volume (slider, 200 % on desktop) / Mute ☐, Mute Soundboard ☐, Disable Video ☐, Change Nickname, Apps >, Ignore, Block / Roles >, Move To > / Mod View, Server Mute ☐ (red), Server Deafen ☐ (red), Disconnect (red), Kick / Copy ID | same order (`voice_panel::user_menu_items` + `voice_items`) | ✅ |
| voice participant, self | local items are Mute ☐, Deafen ☐ (no volume, soundboard or video) at the head of the nickname group; Server Mute / Deafen still shown with the permissions; no Disconnect; Move To lists channels I can join and joins them (`selectVoiceChannel`) | self had no voice moderation, no Move To, and an extra "Voice Settings" group | ✅ fixed |
| Server Mute / Deafen / Disconnect conditions | target in a voice channel; Mute needs MUTE_MEMBERS, Deafen DEAFEN_MEMBERS, Disconnect MOVE_MEMBERS and not self; Move needs MOVE_MEMBERS for others | same, Disconnect no longer offered on self | ✅ |
| Remove Timeout (`csKeta`) | replaces Timeout when the member is timed out | not drawn: the member model has no timeout date yet | ❌ |
| "je suis en vocal" variants | not read live: the user was using the client (route changed during the pass, then joined a call), so the test voice channel join was not done; conditions above come from the JS | | ⚠ |
| DM / group DM rows | not re-read live (needs the @me route); last read 23/11 (`vanilla-menu-dm` / `-group-dm`), DM user menu chunk `9797d9965e471d1d.js` unchanged | | ⚠ |
