# Widgets

Widgets show a piece of system state, with a tooltip on hover and an action on a left click. Pick
them in `jtb.json`, in the order they should appear:

```json
"widgets": ["language", "network", "volume", "battery"]
```

An entry is a name, or `{ "name": "battery", "place": "tray" }`. `place` is `bar` (the default: on
the bar itself, in one row above the notification area on a side bar, between the task buttons and
the tray on a top or bottom bar) or `tray` (a small cell in the notification area of the tray bar,
left of the clock). A change applies while the bar runs. An unknown name is skipped and logged as
`widgets: unknown widget`.

| Name | Shows | Click |
|---|---|---|
| `battery` | the battery level as the stock glyph with the percentage; green while charging, red at 15% or less; hidden on a machine without a battery | opens the Windows quick settings |
| `language` | the three letter code of the input language of the foreground window, such as `ENG` | opens a menu of the installed keyboard layouts; the choice is applied to the window that had the focus |
| `network` | Wi-Fi signal bars, a plug for a cable, a globe when offline | opens the Windows quick settings |
| `volume` | the speaker glyph for the level, or the muted one, with the percentage | opens the Windows quick settings |

Planned: a calendar on the clock, VPN, memory and CPU, weather.

## How they are built

A widget implements `Widget` in `crates/jtb-app/src/widgets.rs`: a name, `state()` (the glyph or text
and tooltip, or none to hide) and `click()`. What each one shows is plain data in
`crates/jtb-core/src/widget.rs` (tested); reading the system is in `crates/jtb-win/src/system.rs`.
The glyph is drawn into a bitmap with GDI (`glyph.rs`, Segoe MDL2 Assets for the symbols). State is
read once a second, the language every 100 ms, and the bitmap is redrawn only when the state changed.

## The quick settings flyout

A click sends Win+A and the flyout (window class `ControlCenterWindow`, hosted by `ShellHost.exe`)
is placed next to the bar that was clicked. Moving it from outside after it is shown flashes at the
default place and leaves its buttons dead for a while, so the placement happens inside
`ShellHost.exe`:

- `crates/jtb-hook` builds `jtb_hook.dll`. The app copies it to `%TEMP%\jtb-hook-<hash>.dll`,
  installs it as a one-shot `WH_CALLWNDPROC` hook on the flyout's thread and the DLL subclasses the
  window. The subclass rewrites the position in `WM_WINDOWPOSCHANGING`, only x on a side bar.
- A loaded DLL stays in `ShellHost.exe`. Each build has its own message name and subclass id, taken
  from the hash in its file name, and the app switches off the target of an older build first.
- The flyout window is parked at its old place until the first open, so the app moves it once while
  it is hidden.
- A second click while it is open sends no Win+A: the click already dismisses the flyout, and
  Win+A would open it again.
- Without `jtb_hook.dll` next to the exe, or if the hook does not attach, the app moves the window
  from outside after it appears (`flyout.rs`). That works but flashes.

Closing `jtb-app.exe` clears the target. A physical Win+A is never moved.
