# Contract: new `jtb.json` keys

Both sections are optional and have defaults, unknown keys inside them are rejected like everywhere else (constitution III).

```json
{
  "start": {
    "enabled": true,
    "command": "C:\\Program Files\\Open-Shell\\StartMenu.exe",
    "args": ["-toggle"],
    "label": "Start",
    "image": "start-button.png"
  },
  "tray": {
    "enabled": false,
    "bar": 0
  }
}
```

- `start.enabled`: draw a Start button as the first button of every bar that has task buttons. Default `false`; the test config turns it on.
- `start.image`: button art in Open-Shell's strip format (frames stacked vertically; normal, hot, pressed; an optional first header row names the frames). Relative paths are read next to the exe. The file is not shipped: the user downloads Open-Shell's `button96.png` (or any strip) and saves it under this name. A missing or unreadable file gives the drawn button (four squares and the label).
- `start.label`: text next to the drawn button; the art button shows no text.
- `start.command`, `start.args`: what a click runs. Validation: command must not be empty when enabled; a missing file is logged, not fatal.
- `tray.enabled`: take the tray role. Default `false`.
- `tray.bar`: index of the bar (in `bars`) that shows the icons. Validation: must be a valid index when enabled.

The old idea of `start_inset` is dropped (research R1); no key is reserved for it.
