# Typing indicator (TypingUsers, inTextChannel)

Source: current-client/css (`base_U9Tvvz`, `inTextChannel_U9Tvvz`, `dot_U9Tvvz`, `cooldownWrapper_bbCgC2`) and the TypingUsers component in the client JS (locale keys lJ9sZX / rB0CUa / StKThj / Q8lUnE).

| element | CSS / JS source | property | value | our code | status |
|---|---|---|---|---|---|
| row position | inTextChannel_.base_ | top / inset-inline | -24px / 0 (above the form, over the chat) | ui/chat/composer.rs form `.absolute().top(-24)` | ✅ |
| form margin | --custom-chat-input-margin-bottom | margin-bottom | --space-xs (no extra 24px) | composer.rs form `.mb(space::xs())` | ✅ |
| row box | base_ | height / font / weight / color | 24px, 12px/16px, 500, --text-subtle | composer.rs `typing()` | ✅ |
| row start | inTextChannel_ / typingDots_ | padding-inline-start | --space-md + 8px | composer.rs `typing()` | ✅ |
| dots | dots_ / dot_ | gap / size / color | 2px, 7px circle, --text-default | composer.rs `dot` | ✅ |
| dot animation | typing-dot-pulse | 1.2s steps(12), delays -1.2/-1.05/-0.9s, .32/.8 ↔ 1/1 at 25% | composer.rs `dot_frame` | ✅ |
| text | inTextChannel_ .text_ | margin-inline-start | --space-md + --space-8 | composer.rs `line` | ✅ |
| names | .text_ > strong | color / weight | --text-default / semibold | composer.rs `bold` | ✅ |
| name choice | JS getName(guild, channel, user) | nick → global name → username | app.rs `typing_name` | ✅ |
| strings | lJ9sZX/rB0CUa/StKThj/Q8lUnE | "… est en train d'écrire...", "A et B sont…", "A, B, et C sont…", "**Plusieurs personnes** sont…" | locales/*.yml chat.typing_* | ✅ |
| overflow fallback | JS qD/0qZ | "Multiple people" when text+56px overflows | — | ❌ not done |
| slowmode | cooldownWrapper_ | right of the same row, padding 3 8, pe 16 | composer.rs `cooldown` | ✅ |
