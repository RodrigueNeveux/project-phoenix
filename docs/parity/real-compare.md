# Live comparison, client vs Phoenix (real account)

Scores from `scripts/compare/compare.py` (`mismatch_pct`, pixels whose difference is above 24/255), client window 1154×835 at devicePixelRatio 1.25, same route, Phoenix rendered offscreen by `real_shots`. Images stay private on /mnt/windata.

## Rail, channel list, user panel, Friends and DM list

### Before (25/11 20:55, `/channels/@me`, Pending tab open in the client, Online in Phoenix, client streaming in voice)

| region | mismatch | mean diff |
|---|---|---|
| title bar | 2.0 % | 0.8 % |
| guild rail | 1.7 % | 1.1 % |
| channel list (DM list + panel) | 10.4 % | 4.1 % |
| header (Friends tabs) | 18.8 % | 3.7 % |
| main (Friends list) | 13.2 % | 5.2 % |
| overall | 11.6 % | |

Most of the remaining channel list difference comes from states that only exist in the client at capture time: the Go Live panel above the voice panel, the active screen share button, the degraded connection icon and the self activity icon in the user panel.

| item | client reference | Phoenix | status |
|---|---|---|---|
| Guild rail positions | rail scan of the capture: every pill and folder within 0.8 px | `server_bar.rs` | ✅ |
| DM row activity line | `subText_TBNqol` > ActivityStatus (`container_HigqCD textXs_HigqCD`): green 12 px activity icon (#45a366), text body-xs/medium, `+N` counter in `--status-positive` when a custom status is shown, `•` then emoji and custom status; row name moves to 8 px from the top (34 px text column centred in 48 px) | `members::dm_sub_text` reuses the member list `sub_text` | ✅ |
| Friends Pending tab | same layout as the client capture once the tab matches (`PHX_TAB=pending`) | `home/friends.rs` | ✅ |
| Voice participants order | VoiceStateStore (module 607567) comparator `${selfStream?"\0":"\x01"}${(member.nick ?? getName(user)).toLowerCase()}\0${userId}`: streamers first, then name, then id as a string | `sidebar::voice_sort_key` | ✅ |
| Channel actions next to the voice timer | f4b2f93f8f92b799.css: `.iconItem_Wx6SnQ{display:none}`, shown on a selected row only with `.iconNoChannelInfo_Wx6SnQ`, on `:hover` `.channelInfo_Wx6SnQ{display:none}`; selected action icons `--icon-strong` | `sidebar::channel_row`: the timer and the user limit pill hide on hover, the actions stay hidden on a selected row that has one | ✅ |
| User panel subtext | `panelSubtextContainer_g6VwXp` > ActivityStatus: 12 px voice or Go Live icon (#45a366), 4 px gap, `•` in `--text-default`, custom status, 12/13 px medium | `members::self_sub_text` when in voice or with an activity, previous text otherwise | ✅ |
| User panel hover roll | `hoverRoll_Dq3ch1`: on hover `.default` fades out and `.hovered` (username) fades in, `transition: all .22s ease` | static swap on the avatar group hover, no slide yet | ❌ slide animation missing |

| Voice participant name vs server tag | `usernameContainer_kQORUk` flex 1 1, min-width 0; `chipletParent_kQORUk` flex 1 0, min-width 0, overflow hidden: the tag is clipped before the name ellipsis | `voice_panel::server_tag` shrinks and clips | ✅ |
| Noise suppression button | `button_9OQtot lookBlank_9OQtot`, background transparent in every state | no background when noise suppression is on | ✅ |
| "New unreads" top bar | module 616740 `renderTopUnread`: hidden when `topMention == null && (isUnreadVisible || bottomUnread != null || bottomMention != null)`; bottom `hide: bottomMention == null && isUnreadVisible`; the scroller ends 12 px under the panels top | `channel_unread_bar::bars` follows the same rule, but on the 25/11 capture Phoenix showed no top bar while the client showed one for #bienvenue (unread in both): Phoenix sees an unread item visible or below that the client does not | ❌ to probe |

### Guild route (25/11 21:20, Project Phoenix campement call view, client in voice and streaming)

| region | mismatch | mean diff |
|---|---|---|
| guild rail | 2.2 % | 0.9 % |
| channel list | 8.2 % | 3.6 % |

## Channel header, chat, composer, member list and their popouts (25/11, wt/live-diff-b)

The client stayed on Friends, then on a voice call view, then on a text channel of the project server; no CDP pass changed its route. Offline passes compare the dumps `vanilla-guild-chat-busy` (1280×700, dpr 1.5), `vanilla-announcement` (1536×840, dpr 1.25), `vanilla-menu-message` and `vanilla-profile-popout` with `real_shots` on the same channel and size (`PHX_GUILD`/`PHX_CHANNEL`, `PHX_SIDEBAR_W=336`, a `node` stub so nothing reads the live client). The live pass (21:53, text channel, 1536×840 at 125 %) read the DOM and computed styles only; `Network.enable` saw no non-GET request.

Scores (`compare.py`, regions rebuilt from the dump DOM):

| view | region | before | after |
|---|---|---|---|
| dump `vanilla-guild-chat-busy` (honeypot) | header | 4.4 % | 4.0 % |
| same | composer | 3.7 % | 3.7 % |
| live text channel (project server, 21:54, int-bin 81ce1c58) | header / composer | — | 2.9 % / 1.4 % |

Messages and member list scores on the dumps are decided by data (two days apart: new reactions, statuses, the Activity panel the dump has and the offline render does not), so they are not used as a measure.

| item | client reference | Phoenix | status |
|---|---|---|---|
| Header topic emoji | `topic_pVbL3l` is markup: `img.emoji` 19×19 at 14 px (1.375 em), no margin | `emoji_text::markup_label` (images, no 1 px margin) | ✅ |
| Composer placeholder width | `placeholder_SHcSVi` 291 px in a 301 px `textArea_` (10 px short of the buttons) | `right(10)` | ✅ |
| Message menu quick reactions | `.wrapper_Qi4I4L{display:grid;gap:4px;grid-template-columns:repeat(4,1fr);justify-items:center;padding:8px}`, `.icon_Qi4I4L` 20×20, button 44×44 radius 8 | 4 equal cells, gap 4, padding 8, emoji 20 | ✅ |
| Member group header | `membersGroupName_86es0b` plain text span with ellipsis, count span `"\xa0— N"` (module 9fb052…) | one ellipsized span + nbsp count | ✅ |
| Reactions spacing | `.container_YUQhib` grid: `padding-top:.125rem`, `grid-row-gap:.25rem`; `.reactions_MmKct8{padding-top:.125rem}` | reactions `pt 4`, `pt 8` after media / embeds / forwards (video→pill 4 → 9 px, client 9) | ✅ |
| Message hover bar | live: `buttonsInner_SY_P2T` right 14, top −16, padding 2, border 0.8 px border-muted, radius 8, `box-shadow: rgba(0,0,0,.14) 0 1px 4px`, 28 px buttons, separator 1×24 border-subtle margin 2 4 | shadow was GPUI `shadow_sm` | ✅ |
| Profile popout « Collection de jeux » | chunk c476f1…: the first game-collection widget renders a breadcrumb (`innerContainer_s6-upr` padding 8 radius 8, title body-xs/medium text-strong, up to 4 icons 24×24 radius 4 gap 4 from `app-icons/{game}/{icon_hash}.png?size=48`, last icon at 50 % opacity with `+N` on `rgba(0,0,0,.6)` when more games) | `profile_games::breadcrumb`, `Game.icon` from `icon_hash`; games fetched like the client in one `GET /games?game_ids=…` per 20 ids (module 920495), the per-game requests never finished before the capture | ✅ 9 games, 3 icons + « +5 », same place as the dump |
| Message menu « Rappelle-moi » | current client `roMu1H` (« Rappelle-moi »); the 23/11 dump said « Créer un rappel » (`mJ3P0N`, now the save-type label) | « Rappelle-moi » | ✅ (no change) |

Open:
- Ellipsis one glyph early on texts with an emoji (composer placeholder « hone… » vs « honey… », member group « Dé… » vs « Dév… »): same box widths and glyph positions within 1 device px, so the budget of `truncate_line_end_shaped` (vendor GPUI patch) is a few px smaller than Chromium's. Left to the text owners.
- Mentioned message background: the token is `--message-mentioned-background-default` = yellow 8 % (computed 0.078 in every dump), but the `vanilla-announcement` capture shows half that on screen (10,6,0 vs Phoenix 19,13,0). Not changed until a live capture of a mentioned message.
- Hover screenshots: `Page.captureScreenshot` of the live client does not show `:hover` states while the window is not repainted (the DOM shows the bar). Hover values above come from the DOM.

### After (25/11 22:10, same guild route, int-bin 4ea4227b)

| region | before | after |
|---|---|---|
| guild rail | 2.2 % | 1.4 % |
| channel list | 8.2 % | 5.6 % |

The remaining channel list difference is live state: the client's Go Live panel (Phoenix is not streaming), the speaking rings and the voice members joining or leaving between the two captures.

### Still open

| item | client reference | status |
|---|---|---|
| DM name with a friend nickname | the 20:55 capture showed "oopi" in the client and "Initial" in Phoenix for the same avatar; the DM row (`tl`, 1b94b2e83f586720.js) reads `RelationshipStore.getNickname(otherUser.id)` | ❌ not checked live (the client left `/channels/@me`) |
| Group DM status dot | the 20:55 capture shows a status dot on the front avatar of a group DM | ❌ DOM not read |
| Soundboard "new sounds" dot | blue dot on the panel soundboard button, aria-label "de nouveaux sons sont disponibles" | ❌ needs the seen-sounds state; do not scroll the soundboard popout (it writes settings-proto/1) |
| Hover roll slide | `.full-motion .default/.hovered { transition: all .22s ease }`, translateY ±107 % | ❌ static swap only |

## Read state bars, DM names, group DM dot, soundboard badge, hover roll (25/11, wt/live-diff-a2)

No CDP pass changed the client's route: it stayed on a text channel of the project server, so the DM list was not in the DOM. The references below come from the cached JS and CSS, the dumps and the 20:55 captures.

| item | client reference | Phoenix | status |
|---|---|---|---|
| Channel list unread bars | rows and bars count a channel as unread only when `hasUnread && resolveUnreadSetting(channel) === ALL_MESSAGES` (module 455234 `Y`); `resolveUnreadSetting` reads the channel flags (1024 all, 512 mentions), then the parent category, then the guild flags (2048 / 4096), then the notification level, and only with the new notification settings; a row with only mentions renders `UNREAD_LESS_IMPORTANT` (no style, no pill, 3d994642c869c81b.js) | `Backend::unreads_all` (inbox `Ctx::unreads_all`), used by `channel_unread_bar` and `sidebar::channel_row`; the bar decision is a pure `pick` with unit tests; `RUST_LOG=phoenix::unread_bar=debug` logs each candidate (unread, muted, unreads_all, mention_count, last_read, place) and the bars, `real_shots` prints it on stderr | ✅ likely cause of the missing top bar (an only-mentions channel counted as a visible unread); to confirm with the probe on the live route |
| Chat "N new messages since HH:MM" bar | `renderStream` (ceb73656db6e6af4.js): shown when `ReadStateStore.getUnreadCount(channel) > 0 && ready`, not tied to the scroll position; text `BctFH/` (same day) / `3wXb9P`, `wvtbbG` / `tHqbtg` (« N+ ») when `isEstimated`; the count is every loaded message newer than the ack, estimated when the ack is not in the loaded page (`rebuildChannelState`, web.b63d846b329d6470.js); clicking the text jumps to `ackMessageId` + 1 (`NewMessagesBarJumpToNewMessages`), « Marquer comme lu » acks | `commands::state::unread_since` at channel open (also for cached pages), bar no longer hidden at the bottom, `jump_to_first_unread`; « Marquer comme lu » goes through `select_channel`, which `real_shots` keeps without ack (`NO_ACKS`) | ✅ unit test |
| DM name with a friend nickname | `computeChannelName` (module 47167): DM `RelationshipStore.getNickname(id) ?? getName(user)`, `@` prefix for the placeholder | `dm_profile_panel::channel_name` in the DM list, chat header, composer placeholder, voice panel, call view, call popout and incoming call | ✅ |
| Group DM name with nicknames | unnamed group DM: `recipients.map(getNickname ?? getName).join(", ")` | still the backend's recipient names; the 20:55 capture also shows a different second name, probably a nickname | ❌ needs to know whether the group has an explicit name (`ChannelExtra.group_name` mixes both) |
| Group DM status dot | `PrivateChannel` (`ej`): status `ONLINE` when any recipient is online, else none; the diagonal facepile (module 954376 / 479abc9ff57878c5.js) then uses `DIAGONAL_FACEPILE_STATUS_32` and the regular 10px status at 22,22 (dump `vanilla-group-dm` #801) | `group_dm::group_avatar_with_status` in the DM list | ✅ |
| Soundboard "new sounds" dot | module 258204: new when `available && userId !== me && soundId > max(2026-11-01, guild joinedAt, lastDismissedObjectId)`; the last value is `PreloadedUserSettings.guilds(3).guilds(1)[guild].guild_dismissible_content_states(12)[870].last_dismissed_object_id(4)`; shown behind the user experiment `2026-10-soundboard-new-sounds-badge` and when the button is enabled; dot `dot_ncMHEz` 6px `--blurple-50` bottom right, icon cut by `masked_ncMHEz` (radius 5 at 100%−3px) | `nav::sounds_dismissed` / `sounds_seen` (READY and `USER_SETTINGS_PROTO_UPDATE`), `SoundboardSound.user`, `soundboard::has_new_sounds`, guild sounds requested once per voice guild; nothing marks them seen | ✅ unit tests; not visible in `real_shots` (no voice there) |
| Hover roll slide | `hoverRoll_Dq3ch1`: `.default` to `translate3d(0,-107%,0)` and opacity 0, `.hovered` from `+107%`, `transition: all .22s ease` | `states::hover_roll` (cubic-bezier .25,.1,.25,1, 220 ms), driven by the avatar group hover | ✅ |
