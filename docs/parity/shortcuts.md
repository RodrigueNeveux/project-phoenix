# Keyboard shortcuts parity

Source: the live 2026 client's shortcuts modal (Ctrl+/), read on 2026-11-24, plus the
`binds:[...]` keybind table in the local web bundles. Implementation:
`crates/phoenix-app/src/shortcuts.rs` (bindings, handlers, Ctrl+/ modal, tests).

## Navigation
| Shortcut | Action | Status |
|---|---|---|
| Ctrl+Alt+Up / Down (also Ctrl+Shift+[ / ], Ctrl+Shift+Tab / Ctrl+Tab) | Previous / next server (Home first, wraps) | ✅ |
| Alt+Up / Down | Previous / next channel (text channels, threads, the joined voice channel; DMs at Home; wraps) | ✅ |
| Alt+Left / Right | Back / forward in history | ✅ |
| Alt+Shift+Up / Down | Previous / next unread channel (skips muted) | ✅ |
| Ctrl+Shift+Alt+Up / Down | Previous / next channel with mentions | ✅ |
| Ctrl+Shift+Alt+V | Jump to the current call | ❌ |
| Ctrl+Alt+Right | Toggle between the last server and DMs | ✅ |
| Ctrl+K | Quick Switcher | ✅ |
| Ctrl+Shift+N | Create or join a server | ✅ |
| Ctrl+1..9 | Nth rail item (not in the modal, but bound in the client) | ❌ |

## Voice and video
| Shortcut | Action | Status |
|---|---|---|
| Ctrl+Shift+M | Toggle mute | ✅ |
| Ctrl+Shift+D | Toggle deafen | ✅ |
| Ctrl+Enter | Answer an incoming call | ❌ |
| Esc | Decline an incoming call | ❌ |
| Ctrl+' | Start a call in a DM or group DM | ❌ |
| Ctrl+Shift+B | Toggle the soundboard | ❌ |

## Chat
| Shortcut | Action | Status |
|---|---|---|
| Shift+Esc | Mark server read | ✅ |
| Esc | Mark channel read (and jump to present) | ✅ |
| Ctrl+Shift+T | Create a group DM | ✅ |
| Ctrl+Shift+Alt+N | Create a thread | ❌ |
| Ctrl+P | Toggle pins popout | ✅ |
| Ctrl+I | Toggle inbox popout | ✅ |
| Ctrl+Shift+E | Mark the inbox's listed channels read | ❌ |
| Ctrl+U | Toggle member list | ✅ |
| Ctrl+E / Ctrl+G / Ctrl+S | Emoji / GIF / sticker picker | ✅ |
| Page Up / Page Down | Scroll the chat | ✅ |
| Shift+Page Up | Jump to the oldest unread message | ✅ |
| Any key | Focus the composer | ❌ |
| Alt+R | Record a voice message | ❌ |
| Ctrl+Shift+U | Upload a file | ✅ |
| Ctrl+Shift+L | Copy channel link | ✅ |

## Focused-message shortcuts
These need keyboard focus on a message row (Up/Down to move it), which we don't have yet.
| Shortcut | Action | Status |
|---|---|---|
| E | Edit | ❌ |
| Backspace | Delete | ❌ |
| P | Pin | ❌ |
| + | Add reaction | ❌ |
| R | Reply | ❌ |
| F | Forward | ❌ |
| S | Dictate (text to speech) | ❌ |
| Ctrl+C | Copy text | ❌ |
| Alt+Enter | Mark unread | ❌ |
| Esc | Focus the composer | ❌ |

## Misc
| Shortcut | Action | Status |
|---|---|---|
| Ctrl+Shift+H, F1 | Get help (opens support site) | ✅ |
| Ctrl+F, Ctrl+Shift+F | Search | ✅ |
| Shift+F10 | Open context menu | ❌ |
| Ctrl+/ (also Ctrl+Shift+/) | Keyboard combos modal (lists the shortcuts above; Esc closes it) | ✅ functionally. The visual still needs a dump: ❌ |
| H H Right N K | Easter egg | ❌ |
| Ctrl+Alt+Shift+W | Lo-fi Wumpus | ❌ |

Known limit: when the composer (gpui-component Input) has focus, its own bindings take
precedence for keys it handles itself, such as Page Up/Down.
