# Data model: Open-Shell coexistence and tray

All of this lives in `jtb-core` and is unit tested on Linux; `jtb-win` and `jtb-app` only translate to and from Windows messages.

## TrayKey
`(owner: isize, id: u32)`: the window handle that owns the icon and the app's icon id. Two adds with the same key are one icon.

## TrayIcon
- `key: TrayKey`
- `tooltip: String`
- `icon: Option<Arc<Icon>>` (same `Icon` type as task icons, premultiplied BGRA)
- `callback_message: u32` (what to send to the owner on a click)
- `version: u32` (callback format: legacy, or version 4)
- `hidden: bool` (the app asked for it not to show)
- `seen: u64` (tick of the last time the owner was found alive)

## TrayIcons
Ordered list in order of arrival.
- `apply(change: TrayChange)`: `Add`, `Modify`, `Delete`, `SetVersion`. Add of an existing key updates it. Modify of an unknown key is an add.
- `drop_dead(alive: impl Fn(isize) -> bool)`: removes icons whose owner is gone.
- `visible() -> impl Iterator<&TrayIcon>`

## StartButton (config, not state)
`start: { enabled: bool, command: String, label: String }`, see `contracts/config.md`.

## TrayConfig
`tray: { enabled: bool, bar: usize }`: index into `bars` of the one bar that shows the tray.

## State transitions
absent -> Add -> shown -> Modify -> shown -> Delete or owner gone -> absent.
