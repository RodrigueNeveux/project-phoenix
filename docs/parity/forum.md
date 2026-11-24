# Forum channels, forum posts and the thread sidebar

Reference: vanilla dumps `vanilla-forum` (forum list), `vanilla-thread-channel`
(forum post opened as a channel), ref-cache `create-thread-sidebar-*` (chat
sidebar frame). Real-data renders: `real_shots` with `PHX_GUILD`/`PHX_CHANNEL`
on a forum, `PHX_OPEN=post[:n]` for the post sidebar.

| element | CSS / dump source | property | value | our code | status |
|---|---|---|---|---|---|
| search/create bar | mainCard_BkGrJJ header_BkGrJJ | h / radius | 48 / 12 | ui/forum.rs render | ✅ (dump box 547x49) |
| "New Post" button | button_F7qvhY sm primary | h / radius / font | 32 / 8 / 14 500 | ui/forum.rs | ✅ |
| guide button | button_F7qvhY sm secondary | 32x32 / radius 8 | | ui/forum.rs | ✅ |
| sort dropdown | sortDropdown_BkGrJJ | h 32, pad 8 8 8 12, radius 24 | | ui/forum.rs | ✅ |
| tag pill | pill_snX-u4 | h 32, pad 0 12, radius 20, 14/600 | | ui/forum.rs tag_pill | ✅ |
| "All" tags button | tagsButton_BkGrJJ | h 32, radius 20, ml 8 | | ui/forum.rs | ✅ |
| post card | container_SOY5H6 mainCard_BkGrJJ | pad 12 10, radius 12, 1px border, gap 16 | | ui/forum.rs post_card | ✅ (101px card, 72px thumb at 1.25 zoom) |
| post title | postTitleText_SOY5H6 heading-lg/semibold | 20/24 600 | | ui/forum.rs | ✅ |
| title colour read/unread | `isNew || hasUnreads` (web bundle, module 767581) | archived → muted; never-opened post past forum ack → strong | | ui/forum.rs is_unread | ✅ (pinned "Médias" now strong like the client) |
| preview author | first message author | name | | ui/forum.rs post_card | ✅ |
| OP tag | botTagOP_feVUtO | --brand-260 bg, --brand-560 text, h 15, pad 0 4.4, radius 4, mt 2.5 | | ui/forum.rs op_tag, ui/chat.rs | ✅ (dump) / render pending |
| thread sidebar frame | chatLayerWrapper_ / container_FrbQJp | w 480, radius 12 0 0, --background-base-lower | | ui/chat/panels.rs thread_panel | ✅ (ref-cache) |
| thread sidebar header | container_ybUK7Z | h 49, pad 8 8 8 20, icon 32/20, body-md/medium --text-strong | | ui/chat/panels.rs | ✅ |
| thread sidebar composer | channelTextArea_ | margin 0 10 10, radius 8, --border-subtle, --chat-background-default | | ui/chat/panels.rs | ✅ |
| thread sidebar header toolbar (bell, pins, threads, more) | not dumped | | | — | ❌ needs a vanilla dump |
| forum post action bar: copy link + jump to top | vanilla-thread-channel buttons_HeWz1C | sm secondary 32x32, radius 8, gap 4 | | ui/chat/panels.rs forum_post_bar | ✅ (dump) |
| forum post action bar: reactions + Following | vanilla-thread-channel reactButtons_HeWz1C | | | — | ❌ follow state not in model |
| opening a forum post by URL (not in active threads) | GET /channels/{id} | | | real_shots falls back to the first channel | ❌ |

## State coverage
- post card: rest ✅, hover (--background-surface-high, --border-strong) ✅; pressed ❌ not measured.
- tag pill: rest / selected ✅; hover ❌ not measured.
| inline new post form | vanilla-forum-new-post container_owUmX8 | title 20/600 h32, body min 80, media tile 78 r12, tag row mt16 mb12 gap 8, small pills h24, divider, controls h40 mt12 | | ui/forum.rs compose_card | ✅ (dump) / render pending |
| announcement "Follow" button | vanilla-announcement followButton_z_gUBg | padding 4 8, mr 8; sm secondary h32 r8 14/500 | | ui/forum.rs follow_button, ui/chat.rs header | ✅ (dump); follow modal ❌ |
| Events modal frame + header | vanilla-events-modal root__KoJTP / flex_hkRWAH | 602x400 r12, header pad 16 24 8, icon 24 mr 8, divider 1x16 mx 16, sm primary | | ui/events.rs | ✅ (dump) |
| Events empty state | container_NkbMg8 | heading-xl 24/32 mb 8, body-sm 14/18, roleTip mt 4 | | ui/events.rs | ✅ (dump) |
| Event card | not dumped (no event on the measured guild) | | | ui/events.rs card | ❌ tokens only |
