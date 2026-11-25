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
