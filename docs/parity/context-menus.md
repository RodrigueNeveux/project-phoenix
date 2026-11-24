# Context menus — parity audit

Reference: vanilla dumps `/mnt/windata/project-phoenix/dumps/vanilla-menu-*.json` (private) and
`menu_M3qk4j` in current-client/css; positioning from ReferencePositionLayer in web.*.js.
Renders: `PHX_OPEN=guildmenu|chanmenu[:n]|catmenu[:n]|usermenu[:n]|msgmenu int-bin/real_shots out.png`.

| menu | dump | items / order / separators | status |
|---|---|---|---|
| server | vanilla-menu-guild | Mark As Read (disabled when read) / Invite / Mute >, Notification Settings > (subtext), Hide Muted ☐, Show All ☐ / Server Settings >, Privacy, Per-server Profile / Create Channel, Category, Event / Move To… > / Security Actions, Leave (danger) / Copy ID | ✅ (render matches) |
| folder | vanilla-menu-folder | Mark Folder As Read / Move To… > / Folder Settings, Close All Folders | ✅ |
| text channel | vanilla-menu-channel | Mark As Read / Favorites > / Invite, Pin to Top, Copy Link / Mute >, Notification > / Move To… > / Edit, Clone, Create Text, Delete (danger) / Copy ID | ✅ (render matches) |
| notification submenu | vanilla-menu-channel-notify | Use Category Default (subtext) ◉, All, @mentions, Nothing | ✅ (fr label "@mentions" fixed) |
| category | vanilla-menu-category | Mark As Read / Collapse ☐, Collapse All / Mute >, Notification > / Move To… > / Edit, Delete / Copy ID | ✅ |
| DM | vanilla-menu-dm | Mark As Read / Pin, Favorites > / Profile, Call, Add Note (subtext), Friend Nickname, Close DM / Apps >, Invite >, Remove Friend, Ignore, Block / Mute @x > / Copy User ID, Copy Channel ID | ✅ |
| group DM | vanilla-menu-group-dm | Mark As Read / Pin, Favorites > / Invites, Edit Group / Mute > / Leave (danger) / Copy ID | ✅ |
| member | vanilla-menu-member-2/-5 | Profile, Mention, Send Message, Call, Add/Edit Note, Friend Nickname / Change Nickname, Apps >, Invite >, Remove Friend, Ignore, Block / Roles > (checkboxes) / Mod View, Timeout, Kick, Ban / Copy ID | ✅ ("Envoyer un message", Edit Note fixed) |
| member in voice | vanilla-menu-member | … Watch Stream … / Roles >, Move To > / Mod View, Server Mute ☐, Server Deafen ☐, Disconnect | ✅ |
| own user | vanilla-menu-member-1 | Profile, Mention / Per-server Profile, Apps > / Roles > / Mod View / Copy ID (no note row) | ✅ (note row removed) |
| message | vanilla-menu-message | 4 quick reactions / Add Reaction, View Reactions / Reply, Forward, Thread / Copy Text, Pin (perm), Bookmark, Remind Me > (30m,1h,4h,tomorrow,Monday,custom), Apps >, Mark Unread, Copy Link, Speak / Remove Reactions >, Remove All, Delete (own or Manage Messages), Report (others) / Copy ID | ✅ except below |
| message: Add Reaction chevron (emoji submenu) | vanilla-menu-message | ours is a plain row opening the picker | ❌ |
| message: View Interaction Info (app messages) | vanilla-menu-message | missing | ❌ |
| voice channel, thread, emoji, link, image menus | — | no vanilla dump yet (user active, no navigation) | ❌ unverified |
| panel | menu_M3qk4j | radius 8, 1px --border-subtle, --background-surface-higher, --shadow-high, min 188 / max 320, scroller 8px | ✅ |
| rows | item_M3qk4j | 34 (text) / 36 (icon) / 52 (subtext), padding 8, 14/18 medium, icon 20 + mr 8 | ✅ |
| edge nudging | web.*.js nudgeTopAlignment | 12px window margin, max-height = window − top − 12 (was 16) | ✅ |
