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

| server tag (message, reply) | 738015 clanTagChiplet_, chipletContainerInner_ | chip | --background-mod-strong, r 4, 0 4px, .72rem, ml 6px (reply 2px, opacity .64), lh 16 | chat.rs server_tag_chip | ✅ |

Still to check: polls, selects, stickers, the slowmode and char counter details.
