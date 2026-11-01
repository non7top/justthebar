# Contract: new `jtb.json` keys

Both sections are optional and have defaults, unknown keys inside them are rejected like everywhere else (constitution III).

```json
{
  "start": {
    "enabled": true,
    "command": "C:\\Program Files\\Open-Shell\\StartMenu.exe",
    "args": ["-toggle"],
    "label": "Start"
  },
  "tray": {
    "enabled": false,
    "bar": 0
  }
}
```

- `start.enabled`: draw a Start button as the first button of every bar that has task buttons. Default `false` until verified on the user's machine, then the test config turns it on.
- `start.command`, `start.args`: what a click runs. Validation: command must not be empty when enabled; a missing file is logged, not fatal.
- `tray.enabled`: take the tray role. Default `false`.
- `tray.bar`: index of the bar (in `bars`) that shows the icons. Validation: must be a valid index when enabled.

The old idea of `start_inset` is dropped (research R1); no key is reserved for it.
