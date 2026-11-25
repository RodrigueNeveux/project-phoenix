# Community server

Progress updates for Phoenix are posted on the project's Discord server, *Discord | Project Phoenix EARLY ACCESS & BETA [UNOFFICIAL]* (guild `1574895284901777448`). The server is in French, so the posts are too.

## Channels

| Channel | Id | Used for |
|---|---|---|
| `📢・annonces` | `1574895965171617982` | One post per milestone: what is new, what comes next, the "not final" warning, up to 4 README screenshots |
| `📸・captures` | `1574895971513532477` | One gallery post per milestone, up to 10 README screenshots with a one-line caption |
| `⚙️・avancés` | `1574898034859581631` | The single tracking message (Done / In progress / Next), edited in place and pinned |

The other channels (rules, roles, discussion, feedback, bugs, support, team, logs) are never written to.

## Tracking message

- `⚙️・avancés`, message `1574898034859581631/1574898068434714644`: the owner's original "Déjà présent / Manque ou partiel" list, now kept as Fait / En cours / À venir. Edit it, never post a second one.

## Rules

- At most one post per channel per milestone. No `@everyone`, no `@here`, no mentions, no replies to members on the owner's behalf, no moderation and no server settings.
- Images come only from the screenshots already reviewed for the README (`docs/screenshots/<date>/`), with the same privacy rules: no profiles, no account pages, third-party names blurred.
- Every post says that the builds and screenshots are not final versions.

## Posting

Posts go through `crates/phoenix-app/examples/community_post.rs`, which uses the real Phoenix REST client with the token from the keyring. It refuses anything but this guild and the three channels above (checked by id and by name), any text over 2000 characters or containing `@everyone`, `@here` or a mention, any image outside `docs/screenshots/<date>/`, and edits or pins of messages the account did not write. Mentions are disabled with `allowed_mentions`.

```sh
cargo build -p phoenix-app --example community_post
EX=target/debug/examples/community_post
$EX check annonces annonces.txt docs/screenshots/2026-11-25/chat-general.png
$EX send annonces annonces.txt docs/screenshots/2026-11-25/chat-general.png
$EX edit avances 1574898068434714644 avances.txt
$EX pin avances 1574898068434714644
```

`check` only runs the GET guards and validates the text and images. To update the tracking message after a milestone, rewrite the three lists from the README status and the `docs/parity/` tables, then `edit` it.

## History

| Date | Channel | Message |
|---|---|---|
| 2026-11-25 | `⚙️・avancés` | `1574898068434714644` (edited and pinned) |
