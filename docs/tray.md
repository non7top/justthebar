# Tray (work in progress)

`"tray": { "enabled": true, "bar": 0 }` makes justthebar take over the notification area role, the way Cairo Desktop does: the app creates its own hidden `Shell_TrayWnd` window above Explorer's, tray apps send their icon messages to it, and it passes everything it does not handle on to Explorer's window. Explorer keeps running.

Current state: the window is created, the icon packets are decoded, and every packet is a line in `jtb.log`. The icons are drawn in the bar `tray.bar` (index in `bars`), just before the clock: one row on top and bottom bars, a grid on side bars. Hovering one highlights it and shows its tooltip, and clicks and hover are passed on to the app that owns the icon. Leave `enabled` false (the default) for daily use.

## What to look for in `jtb.log`

- `tray: role taken` when the app starts or the config is saved with the tray on.
- `tray: message=0 owner=<exe> ... icons=N` once per icon that an app (re)sends, with `icon=true` when the icon image could be read. Apps resend after the app sends the `TaskbarCreated` broadcast, so icons of running apps show up within a few seconds.
- `tray: raised our Shell_TrayWnd` if something else got above our window.
- The bars must still reserve their space and keep working: our own bar registrations pass through the tray window on their way to Explorer.

## Getting out of trouble

Ending `jtb-app.exe` normally gives the role back and tells every app to send its icons to Explorer again. If the app was killed, run `9-restore-explorer.cmd` (it runs `jtb-app.exe --restore-explorer`), which restores the taskbar and sends the same broadcast.
