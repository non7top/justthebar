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
| `battery` | a fat battery lying on its side, drawn by the bar as tall as the ink of the Wi-Fi glyph and filled from the left to the level, with the number on it (the percent sign is in the tooltip): green from 50%, yellow from 25%, orange above 15%, red at 15% or less; blue with a bolt while the power cable is connected (charging, or plugged in and full); hidden on a machine without a battery | opens the Windows quick settings |
| `language` | the two letter code of the input language of the foreground window, such as `EN` (three letters for a language that has no two letter code) | opens a menu of the installed keyboard layouts; the choice is applied to the window that had the focus |
| `network` | Wi-Fi signal bars, a plug for a cable, a globe when offline | opens the Windows quick settings |
| `volume` | three bars drawn by the bar, lit from the left by the level in red, green and orange, or all dim with a red slash when muted or at 0% (the percentage is in the tooltip) | opens the Windows quick settings |

Planned: a calendar on the clock, VPN, memory and CPU, weather.

## Layout on a side bar

The widgets sit in one row. Each cell is as long as its content: the measured width of the label plus
the icon. Icons and text are scaled together to fill the bar, from 75% to 150%; if the row is too
wide even at 75%, the labels move under the icons, and then only the icons are shown
(the tooltip has the rest). A label is never cut to dots. The row is stretched to the width of the
bar with the content centered in each cell. Only when even the icons do not fit does the row wrap
to a second one.

## How they are built

A widget implements `Widget` in `crates/jtb-app/src/widgets.rs`: a name, `state()` (the glyph or text
and tooltip, or none to hide) and `click()`. What each one shows is plain data in
`crates/jtb-core/src/widget.rs` (tested); reading the system is in `crates/jtb-win/src/system.rs`.
The battery and the volume bars are drawn from shapes on one grid (`crates/jtb-core/src/widgeticon.rs`, tested), the
other symbols are glyphs drawn into a bitmap with GDI (`glyph.rs`, Segoe MDL2 Assets). State is
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
