# Widgets

Widgets show a piece of system state in the notification area of the tray bar (`tray.bar`), left of
the clock and after the tray icons: one small cell each, with a tooltip on hover and an action on a
left click. Pick them in `jtb.json`, in the order they should appear:

```json
"widgets": ["language", "battery"]
```

A change applies while the bar runs. An unknown name is skipped and logged as `widgets: unknown
widget`.

| Name | Shows | Click |
|---|---|---|
| `battery` | the battery level as the stock glyph, charging variant while on charge; hidden on a machine without a battery | opens Settings, battery |
| `language` | the two letter code of the input language of the foreground window, such as `EN` | opens Settings, language |

Planned: `volume`, `network`, a calendar on the clock, VPN, memory and CPU, weather.

## How they are built

A widget implements `Widget` in `crates/jtb-app/src/widgets.rs`: a name, `state()` (the glyph or text
and tooltip, or none to hide) and `click()`. What each one shows is plain data in
`crates/jtb-core/src/widget.rs` (tested); reading the system is in `crates/jtb-win/src/system.rs`.
The glyph is drawn into a bitmap with GDI (`glyph.rs`, Segoe MDL2 Assets for the symbols) and
handed to the bar as a tray icon, so layout, hover, tooltips and clicks are shared with the tray.
State is read once a second and the bitmap is redrawn only when the state changed.
