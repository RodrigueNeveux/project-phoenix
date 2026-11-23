# Chat parity audit (current client CSS)

CSS under ~/discord-phoenix-ref/current-client/css. Tokens are from `theme::generated::idx` (`tk(idx::X)` in chat.rs).

| element | CSS source | property | CSS value | our code | status |
|---|---|---|---|---|---|
| channel header | 436803 container_ybUK7Z | height / padding-start / border | 49px / --custom-message-margin-horizontal / 1px --border-subtle | ui/chat.rs header() | ✅ |
| header icons | 436803 clickable_/selected_ .icon_ | color | --icon-muted / --icon-strong | chat.rs header() | ✅ |
| header divider | 436803 divider_ybUK7Z | size, margin | 1x24, 0 4px | chat.rs header() | ✅ |
| message wrapper (cozy) | 738015 cozy_.wrapper_ | padding | 2px, start 72px, end 16px | chat.rs message() | ✅ (pr was 48) |
| compact wrapper | 738015 compact_.wrapper_ | padding-start | 5rem | chat.rs message() | ✅ |
| group start | 524434 groupStart_ (.group-spacing-16) | margin-top | 1.0625rem | chat.rs gap 17 | ✅ |
| avatar | 738015 avatar_ | size / left / top | 40px / 16px / 4px | chat.rs | ✅ (top was 2) |
| username | 738015 username_ | font / color | 1rem medium 1.375rem / --text-strong | chat.rs author_name | ✅ (fallback was text-default) |
| timestamp cozy | 738015 cozy_ .timestamp_, timestampInline_ | size/color/margin | .75rem medium --chat-text-muted, ml .25rem | chat.rs | ✅ (ml was 8) |
| timestamp alt / compact | 738015 timestamp_.alt_ / compact_ | size/color/width | .6875rem --text-muted, 56px | chat.rs | ✅ |
| APP tag | 332923 .rem_.botTag_, botTagRegular_ | h/pad/radius/bg | .9375rem / 0 .275rem / 4px / --background-brand | chat.rs bot_tag | ✅ (radius was 3) |
| reply line | 738015 repliedMessage_ | font / lh / mb | 14px / 1.125rem / 4px | chat.rs reply | ✅ |
| reply compact | 738015 compact_ .repliedMessage_ | margin/padding start | -24px / .625rem | chat.rs reply | ✅ |
| reply spine | 738015 messageSpine_:before | color/geometry | 2px --spine-default, r 6px, top 50% | chat.rs reply | ✅ |
| reply avatar | 738015 replyAvatar_ | size/margin | 16px / 4px | chat.rs | ✅ |
| hover bg | 524434 message_:hover | background | --message-background-hover | chat.rs | ✅ (was mod-muted) |
| mention | 524434 mentioned_ | bg / hover / bar | --message-mentioned-background-default/-hover, 2px --icon-feedback-warning | chat.rs | ✅ |
| replying | 524434 replying_ | bg / hover / bar | --message-highlight-background-default, --brand-10a, --brand-500 | chat.rs | ✅ (was mention colours) |
| hover toolbar | 28636 popover_, 319714 button_ | bg/border/radius/pad | --background-surface-high, 1px --border-muted, 8px, 2px; buttons 24+2*2, r 6px, icon 20px, hover --interactive-background-hover | chat/message_ext.rs toolbar | ✅ |
| toolbar separator | 28636 separator_ | size | 1x24, margin 2px 4px, --border-subtle | message_ext.rs | ✅ |
| toolbar position | (inline style in JS) | top/right | not in CSS | -16 / 16 | ✅ (no CSS rule) |
| edited | 738015 edited_ | size | .625rem normal | chat.rs edited_marker | ✅ |
| day divider | 524434 divider_, 560338 divider_/content_ | margins/line/label | 16/14px, 24/8px, --border-subtle; 12px semibold lh 13, pad 2 4, r 8, --text-muted | chat.rs day_divider | ✅ |
| new divider | 560338 isUnread_/endCap_ | color/cap | --background-feedback-notification; 13px, 10px bold, r 0 4 4 0, top -7 | message_ext.rs new_divider | ✅ |
| headings | 44141 markup_ h1-h3 | size/margins | 1.5/1.25/1rem, 16 0 8, first 8 (h1,h2) / 4 (h3) | chat.rs block | ✅ |
| lists | 44141 markup_ ul/li | indent/mb | 16px / 4px | chat.rs block | ✅ |
| blockquote | 44141 blockquoteContainer_ | bar/pad/margin/color | 4px r4 --spine-default, 12/8px, 4px, --text-subtle, max 90% | chat.rs, richtext.rs | ✅ |
| inline code | 44141 markup_ code | bg | --background-code | richtext.rs | ✅ (border and 85% size not possible on a text run) |
| code block | 44141 pre/code, 189423 .hljs | bg/border/pad/font | --background-base-lower, 1px --border-normal, .5em, .875rem/1.125rem, --text-code, mt 6, max 90% | chat.rs block | ✅ |
| subtext | 44141 markup_ small | font/color | .875rem / 1.20313rem / --text-subtle | chat.rs, richtext.rs | ✅ |
| timestamp pill | 44141 timestamp_ | bg | --background-mod-normal | richtext.rs | ✅ (radius not possible on a run) |
| mention pill | 738015 .mention | colors | --mention-background / --mention-foreground | richtext.rs | ✅ (radius not possible on a run) |
| links | 44141 markup_ a | color | --text-link | richtext.rs | ✅ |
| embed | 198496 embedFull_, grid_, gridContainer_ | bg/border/pad/max | --background-surface-high, 1px --border-subtle, 4px --border-normal, pad 2 16 16 12, 516px | chat.rs embed | ✅ |
| embed parts | 198496 embedAuthorIcon_/FooterIcon_/Thumbnail_/Media_ | sizes | 24px, 20px, 80px ml16 mt8, media mt16 r4 | chat.rs embed | ✅ |
| embed provider | 198496 embedProvider_ .embedLink_ | color | --interactive-text-default | chat.rs | ✅ |
| image attachment | 635014 oneByOneGridSingle_ | radius/max-h | 8px / 350px (max-w 550) | chat.rs media::MAX_* | ✅ |
| reactions | 17256 reaction_/reactionInner_/reactionCount_ | pill | r 8, 1px border, pad 2 6, emoji 20 (my 2), count 14 semibold --text-subtle ml 6, gap 4 | message_ext.rs reactions | ✅ |
| reaction states | 17256 :hover / reactionMe_ | colors | --reaction-background-hover/--reaction-border-hover; --reaction-*-reacted-default | message_ext.rs | ✅ |
| component buttons | --custom-button-button-sm-* | size | 32 x min 60, r 8 | ui/components.rs button | ✅ |
| composer | 264141 form_, 962953 channelTextArea_ | pad/bg/border/radius | --space-xs (8px), --chat-background-default, 1px --border-muted, 8px | chat/composer.rs | ✅ (px was 16, no border) |
| composer inner | 962953 inner_/attachButton_/buttons_/button_ | spacing | pl 15, attach -14/10, gap 8, mr 6, 32px buttons, 20px icons | composer.rs | ✅ (icons were 24) |
| placeholder | 962953 textArea_::placeholder | color | --text-muted | composer.rs | ✅ |
| reply bar | 841136 replyBar_, closeButton_, mentionButton_, separator_ | bg/pad | transparent, text ml 16, pad 8 12, sep 1x20 --border-subtle | composer.rs | ✅ |
| typing | 56501 base_U9Tvvz | font/color | 12px/16px 500, --text-subtle, h 24, gap 4, dots gap 2 | composer.rs typing | ✅ |
| welcome | 274698 container_/emptyChannelIcon_/description_ | margin/icon/color | 16px; 68px mt16 --background-mod-muted; --text-default | chat.rs intro | ✅ |

| server tag (message, reply) | vanilla-dm #1270 / #1424 clanTagChiplet_ chipletContainerInner_ | chip | --background-mod-strong, r 4, 0 4px, .72rem, h 15, margin -2 0 0 6 (reply -2 4 0 2, opacity .64) | chat.rs server_tag_chip | ✅ (reply mr 4 + h 15 added) |

Still to check: polls, selects, stickers, the slowmode and char counter details.

## Live DOM dump references (2026-11-23)

Dumps from the real client (`/mnt/windata/project-phoenix/dumps`, private, not committed). The user's client runs
`html.density-cozy`, so `--space-xs/md/xl` are 10/20/30px (chat.rs `SPACE_XS/SPACE_MD/SPACE_XL`). Node numbers are
indices in the dump JSON (`tools/tree.py <dump> <index>` in ~/discord-phoenix-ref).

| element | dump node | values | our code | status |
|---|---|---|---|---|
| header section | -channels-N-N+ #1040, -channels-me-N+ #1088 | padding 8 8 8 20, h 49, border-bottom --border-subtle | chat.rs header() | ✅ |
| channel icon | #1043/#1044 | 20px glyph in 20×32, margin 0 11 0 -1, --channel-icon | header() | ✅ |
| DM avatar | me #1092 | 20px, margin-inline-end 8px | header() | ✅ |
| title | #1047 | 16px/20px medium, --text-strong, mr 8 | header() | ✅ (was semibold, --header-primary) |
| topic dot + topic | #1049, #1050 | dot 4px m 0 4; topic ml 8, 14px/18px medium --text-muted | header() | ✅ (was a 1×24 divider) |
| toolbar | #1054 | gap 10, padding-left 10, 32px buttons, 20px icons | header() | ✅ |
| guild toolbar order | #1055–#1061 | threads, notifications, pins, member list | header() | ✅ |
| DM toolbar order | me #1101–#1109 | voice call, video call, pins, add to DM, user profile (selected) | header() (icons `phone-call`, `video-call`, `add-to-dm`, `user-profile`) | ✅ (icons need provider names) |
| search bar | #1063–#1073 | ml 4, 244×32, r 8, --input-background-default, 1px --input-border-default, editor px 6 14px/22px medium, 24px icon box p 4 | header() | ✅ (was 160px) |
| search placeholder | #1066 aria | "Search <guild name>" / "Search <username>" | header() | ✅ |
| message row | #1184 | padding 2 30 2 80 (--space-xl, left-content-cozy), radius 0 4 4 0 | message() | ✅ (was 2 16 2 72) |
| avatar | #1200 | 40px, left 20 (--space-md), top 26 + margin 2 with reply | message() | ✅ |
| reply line | #1185 | h 20, 14px/18px, mb 4 | message() reply | ✅ (was 18) |
| reply spine | #1186 | left -41, top 9, 37×13, 2px --spine-default, r 6 top-left | message() reply | ✅ |
| server tag chip | #1190 (reply) / #1206 (header) | padding 0 4, r 4, --background-mod-strong, badge 12/14px, 12px semibold text | server_tag_chip() | ✅ (header badge was 12) |
| cozy timestamp | #1215 | ml 4, 12px medium --chat-text-muted | message() | ✅ |
| alt timestamp | #1252 | 56px wide, 11px medium --text-muted | message() | ✅ |
| mention highlight | #1184 .mentioned_ | --message-mentioned-background-default | message() | ✅ |
| day divider | #1181/#1182 | margin 24 14 8 16, content 12px/13px semibold p 2 4 r 8 --text-muted | day_divider() | ✅ |
| image attachment | #1388–#1394 | container gap 4 padding 2 0; media radius 8 | attachment() | ✅ |
| composer form | #1432 | padding-inline --space-xs (10) | composer.rs | ✅ (was 8) |
| text area | #1435/#1437 | h 56, r 8, --chat-background-default, 1px --border-muted, mb 10 | composer.rs | ✅ (was 44) |
| attach button | #1438–#1442 | inner pl 15, wrapper p 12 14, 32px r 8 margin 0 10 0 -16, PlusLargeIcon 20 | composer.rs | ✅ (was CirclePlus) |
| text placeholder | #1445 | padding 17 0, 16px/22px, --channel-text-area-placeholder | composer.rs | ✅ |
| buttons | #1452–#1480 | gap 8, 32px r 8, 20px icons; gift, GIF, sticker, emoji, app launcher; 12px to edge | composer.rs | ✅ (app launcher added) |
| translate button | #1453 `vc-trans-*` | Vencord plugin, absent from the vanilla dumps | — | removed from the audit |
| typing | CSS base_U9Tvvz | absolute bottom 1, h 24, inset 16; dots 7px --text-default opacity .32 | composer.rs typing() | ✅ |

## Vanilla client re-dump (2026-11-23, Vencord off)

Dump `vanilla-guild-text` (guild text channel with embed, reactions, reply). No `vc-*` nodes in the vanilla dumps: the Vencord additions (translate button, platform indicators, voice speaker icons, message decorations) are gone.

| element | dump node | values | our code | status |
|---|---|---|---|---|
| reply order | #1280..#1291 | avatar 16 (mr 4), then APP tag (`botTagCompact_`, mr 4), then username 14/500, then preview | chat.rs `reply` | ✅ (tag was after the name) |
| reply "@" | #1287 | no "@" when the reply didn't ping | chat.rs `reply` (`msg.reply_mentions_author`, set in convert.rs from `mentions` ∋ replied author) | ✅ |
| reply placeholder | #1291 `repliedTextPlaceholder_` | "Clique pour voir le message", italic, when the replied message has no text | chat.rs `reply` | ✅ (new) |
| edited marker | #1211 `edited_2i8Bfm` | 10px/10 400 `#73747d`, own line after an embed | chat.rs `edited_marker` | ✅ |
| reactions | #1275 `reactionBtn_ forceShow_` | add button always visible: padding 5 7, r 8, 1px transparent, `--reaction-background-default`, 18px icon | message_ext.rs `reactions` | ✅ (was hover-only, 20px) |
| reaction pill | `reaction_` | r 8, padding 2 6, emoji 20, count 14 semibold | message_ext.rs | ✅ |
| discovery notice | vanilla-guild-text #1101 `bar_8PBDGr` / #1103 text / #1105 close | --background-feedback-mod-info, 1px --inlinenotice-border-info, padding 4 8 4 16, gap 8, 14/18 --text-default, underlined "En savoir plus", 32px close 18px glyph | chat.rs `discovery_notice` | ✅ (compare honeypot (Discovery guild) 23.0→22.1 % overall, messages 25.5→22.7; anchor arrow `arrow_8PBDGr` 16×10 implemented (own path, centred on header label #1061, top = bar − 9) — ✅ vanilla-guild-text #1102 `arrow_8PBDGr` 16×10 at x657 y72, same box as ours; live compare 20261123-152127 on Aperture #honeypot: arrow under "Mode Découverte", header 8.1 %, overall 18.9 %) |
| header topic | vanilla-guild-text #1060 dot_GxzgQj (ml 4 mr 12), #1062 label 14/18 500 --text-muted, #1068 leadingBar_ 1×16 --border-subtle ml 8 | "Mode Découverte" before the topic | chat.rs `header` | ✅ (side-by-side match on honeypot; header region 10.6 % unchanged, dominated by other glyphs) |
| reply "@name" for my own messages | real compare 2026-11-23 (guild text) | replied author nick | `@vohan` (my guild nick) | chat.rs reply header | ❌ own member now injected into GuildMembers (convert.rs list_update) + nick reused from loaded messages by the same author; compare 20261123-152617 still shows the global name (the author isn't me but another member outside the synced range) |
| message group vertical rhythm | real compare 2026-11-23 | group spacing | ~5px less per group than ours (cumulative drift) | chat.rs | ❌ cause found: cozy repliedMessage_ is 18px (vanilla-guild-text-groups #2624), we used 20 → fixed; residual ~1px per reply group remains (messages 13.3→13.4%) |
| member activity "Dans un salon vocal" | real compare 2026-11-23 | member row subtitle | voice-state activity line | members.rs | ❌ "Dans un salon vocal" text added (dump #3441), but real_shots has no voice states for these members so it isn't visible in the compare yet |
| avatar decorations in messages/member list | real compare 2026-11-23 | decoration overlay | shown | chat.rs / members.rs | ❌ drawn (avatarDecoration_2i8Bfm 48px at avatar−4, #2641; member list 120%); not yet verified in a compare |
| forum post list | dump vanilla-forum (list_BkGrJJ, container_SOY5H6 mainCard_BkGrJJ, archivedDividerRow_BkGrJJ) + compare 20261123-152703 | cards: padding 12/10, radius 12, --background-base-low, 1px --border-subtle, 16 gap; title 20/24 semibold (muted when read), preview 14/18, footer count+time, 72px thumb; all active pages then "Older posts" (threads/search archived=true) paged on scroll | 1 card, no section | all cards + Anciens posts, side-by-side match; forum column mismatch 30.3%→18.7% (composer box), messages box 16.3%→1.4% | ui/forum.rs post_card/render | ✅ |
| forum header bar | dump vanilla-forum header_BkGrJJ, container_owUmX8, button_F7qvhY sm | h48, radius 12, --input-background-default, 20px semibold placeholder, guide 32px secondary + "Nouveau post" 32px brand | 40px plain input | matches dump | ui/forum.rs render | ✅ |
| forum tag/sort row | dump tagsContainer_BkGrJJ, sortDropdown_BkGrJJ, divider_BkGrJJ, pill_snX-u4, tagsButton_BkGrJJ | padding 16 0 4; pills h32 radius 20/24, --control-secondary-background-default; divider 2x24 mx16 | 24px pills | matches dump (sort icon is a placeholder) | ui/forum.rs render | ✅ |
| forum pinned badge | dump pinIcon_nLtnFF | 22px circle --brand-500 | none | thread flag 1<<1 → badge, pinned first, date footer | ui/forum.rs post_card | ✅ |
| media channel grid | no media channel (type 16) in any of the user's 44 guilds' visible sidebars (CDP scan 2026-11-23: aria-labels are only salon textuel/vocal/forum/annonces) | — | tiles 260px | not verified | ui/forum.rs media_tile | ❌ blocked: nothing to dump |
| member list Activity section gate | client bundle, member list groups hook: `showFeed = guildFeedEnabled(guild) && !channel.isForumChannel()`; guildFeedEnabled = feature ACTIVITY_FEED_ENABLED_BY_USER → true, ACTIVITY_FEED_DISABLED_BY_USER → false, else memberCount < 10000; isForumChannel = GUILD_FORUM only (media keeps it); header chevron only when count > 3 | hidden next to forums | forum compare 20261123-154018: section gone, member list 25.7%→20.8%; text channel of same guild shows "Activité — 34" on Discord (CDP) | ui/members.rs activity_section, Backend::activity_feed_enabled | ✅ |
| forum Sort & View icon | bundle: sortDropdown button → ArrowsUpDownIcon size xs + text + ChevronSmallUp (open) / Down, width 20 | icons | ArrowsUpDownIcon 16, chevron 20 | — | ui/forum.rs, icons/names.rs "sort"/"chevron-up" | ✅ |
