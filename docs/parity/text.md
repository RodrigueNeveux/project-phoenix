# Text rendering parity

Values come from the live vanilla client (CDP computed styles, cached in
`/mnt/windata/project-phoenix/ref-cache/`) and the CSS bundles in
`~/discord-phoenix-ref/current-client/css/`. Scores come from `scripts/compare/compare.py`
(mismatch %, lower is better). The dumps and renders are private and not in the repo.

| element | CSS source | property | CSS value | our code | status |
|---|---|---|---|---|---|
| UI font | `--font-primary` | font-family | gg sans (from the local Discord cache) | fonts/mod.rs `ui_family` | ✅ |
| Container text display | `root_ sizeReduced_` | font-size / line-height | .875rem / calc(--chat-markup-line-height * .875) = 14 / 19.25 | ui/components.rs `markdown_sized` | ✅ (messages 8.9 → 8.0) |
| Container subtext `-#` | `sizeReduced_ small` | font-size / line-height | .75rem / 16.5px | ui/components.rs `markdown_sized` | ✅ |
| Subtext outside containers | `markup_ small` | size / line-height / color | .875rem / 1.20313rem / --text-subtle | ui/components.rs, ui/chat.rs | ✅ |
| Heading in container | `markup_ h2` (not reduced) | size / line-height / margin | 20 / 27.5 / 0 | ui/components.rs | ✅ |
| Container width | `container_rc97-L` in a flex row | width | content width | ui/components.rs `Comp::Container` | ✅ |
| Emoji in container text | `markup_ .emoji` | size | 1.375em (19.25 at 14px) | ui/chat/message_ext.rs `rich_with_emoji_scaled` | ✅ |
| Activity card name | `userName_g7esM5` | color / font / max-width | --channels-default / 14px, 1.2857, 500 / 85% | ui/members.rs activity_section | ✅ |
| Activity card name style | `name_Wo4EpK` + font class | font, letter-spacing, gradient | style font, .01-.04em, gradient | fonts::styled_name + name_text (first stop only) | ⚠️ no gradient fill |
| Member row height | `member_Iy0c7A` | height | 42 (one line) / 44 (with sub line) | ui/members.rs `member_row_in` | ✅ |
| Member list content width | `content_tqapBM` inside `members_86es0b` | width | 260 of 268 (thin scroller gutter) | ui/members.rs list `.pr(8)` | ✅ |
| Member group header | `membersGroup_86es0b` | padding / height | 24 4 4 20 / 46 | ui/members.rs `group_header` | ✅ |
| Letter-spacing in chat, sidebar, members, headers | – | letter-spacing | normal everywhere on the main screen (live scan); only styled display names have tracking | fonts::name_text | ✅ |
| Ellipsis of letter-spaced styled names | `userName_` text-overflow | ellipsis | "…" | name_text lays out glyph by glyph: clipped, no "…" | ❌ |
| Channel list label baseline | `name_` 16/20 | baseline | – | side-by-side crop: same baseline | ✅ |

Scores on the same guild/channel:
- First session: messages 8.9 → 8.0, channel list 7.5 → 6.5, overall 11.0 → 10.4.
- Second session: member list 26.6 → 21.5.
