# Input QA

Static renders prove a view looks right, not that it reacts. The input QA runs
the offscreen `real_shots` window on the real account (read-only backend, no
acks) and feeds it the same `PlatformInput` events the Linux platform sends:
wheel notches as `ScrollDelta::Lines` (3 lines a notch, like Wayland
`AxisValue120`), touchpad swipes as a burst of `ScrollDelta::Pixels`, mouse
down / up with a click count, moves with a pressed button for drags, and
keystrokes through `window.dispatch_keystroke` (so key bindings and the text
input handler both see them). Between steps it reads app state and compares.

```sh
cd /mnt/windata/project-phoenix/compare/qa-input
PHX_AGENT=1 PHX_W=1280 PHX_H=800 PHX_SCALE=1.5 PHX_GUILD=1369382919948210248 \
PHX_CHANNEL=1403447573389381652 PHX_SCRIPT=~/discord-phoenix/scripts/qa/basic_input.qa \
flock /tmp/phoenix-render.lock phoenix-nice ~/discord-phoenix/int-bin/real_shots qa.png
```

Every check prints `QA OK [step] ...` or `QA KO [step] ...`, then
`QA summary ok=N ko=M` and the list of shots (`qa-<name>.png`).
`PHX_SCRIPT_STRICT=1` turns a failed check into a failed run.

The mock counterpart, `ui/input_qa_tests.rs`, sends the same kind of events to a
`VisualTestContext` window on demo data and runs with `cargo test` in the
integrator.

## Script commands

Points are window coordinates in logical px (`x,y`) or anchors:
`@chat`, `@sidebar`, `@members`, `@guilds` (list viewports), `@settings` (the
settings scroll area), `@window`. `.tl .tr .bl .br .t .b .l .r` pick a corner
or edge instead of the centre, `#n` picks list item `n` (`#-1` = last), and
`+dx,dy` offsets the point: `@chat#-1.tl+30,20`, `@chat.b+0,-24`.

| command | effect |
| --- | --- |
| `step <label>` | names the checks that follow |
| `wait <ms>` | drives frames (animations, springs, network) |
| `move <pt>`, `hover <pt> [ms]` | mouse move |
| `down <pt> [left\|right\|middle]`, `up <pt> [btn]` | one press or release |
| `click <pt> [btn]`, `dblclick <pt>` | move, then down + up (click count 1, 2) |
| `drag <from> <to> [steps]` | press, moves with the left button held, release |
| `wheel <pt> <notches>` | mouse wheel, positive = up (towards older messages) |
| `lines <pt> <dy>`, `pixels <pt> <dy>` | one raw scroll event |
| `touchpad <pt> <px> [events]` | a touchpad swipe split in pixel events |
| `key <k>...` | keystrokes (`pageup`, `ctrl-k`, `escape`, `shift-tab`) |
| `type <text>` | one keystroke per character |
| `hold shift\|ctrl\|alt\|none` | modifiers for the following mouse events |
| `action pageup\|pagedown\|top\|bottom` | dispatch the action itself (no key) |
| `select <channel id>` | setup only: open a channel without clicking |
| `region <name> x,y,w,h` | a screen rectangle for `px:<name>` |
| `shot <name>` | capture `<out>-<name>.png` |
| `print <probe>...`, `mark <probe>...` | print, or remember for `changed` / `up` / `down` |
| `expect <probe> <op> [value]` | one check |
| `until <ms> <probe> <op> [value]` | wait until the check holds, then report it |

Operators: `== != < > <= >=`, `~` (contains), `changed` / `same` (against the
mark), `up` / `down` (greater / smaller than the mark by more than the value,
0.5 by default).

Probes: `chat_y` (px from the top of the message list), `chat_end` (px above the
bottom), `chat_max`,
`chat_item`, `chat_items`, `at_bottom`, `sidebar_y`, `members_y`, `guilds_y`,
`settings`, `settings_y`, `channel`, `guild`, `msgmenu`, `navmenu`,
`usermenu`, `menus`, `popout`, `profile`, `modal`, `switcher`, `composer`,
`ac` (autocomplete rows), `video_t` (current time of the clicked video, -1
when none), `fullscreen`, `px:<region>` (hash of the region's pixels, for
"the video frame changes"), `at:<point>` (prints where a point resolves).
