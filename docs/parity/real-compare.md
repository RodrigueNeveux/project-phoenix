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

### Guild route (25/11 21:20, Project Phoenix campement call view, client in voice and streaming)

| region | mismatch | mean diff |
|---|---|---|
| guild rail | 2.2 % | 0.9 % |
| channel list | 8.2 % | 3.6 % |
