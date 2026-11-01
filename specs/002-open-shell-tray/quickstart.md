# Quickstart: checking Open-Shell and tray on Windows

Prerequisites: Open-Shell installed (Start button and Win key work with the Explorer taskbar visible), `make build && make deploy`, `jtb.json` with `"start": {"enabled": true}` and later `"tray": {"enabled": true, "bar": 0}`.

1. **Start button**: run `4-bars-from-config.cmd`. Click the Start button on the bar: the Open-Shell menu opens. Note where it opens; press the Win key and compare. (spec US1)
2. **Tray off**: with `tray.enabled` false nothing about the tray changes; Explorer keeps its tray.
3. **Tray on**: set `tray.enabled` true, save. A tray app's icon (for example a sync client) shows on bar 0 within a second. Hover for the tooltip, right click for the app's menu. (US2, US3)
4. **Restart a tray app**: its icon disappears and comes back on the bar. No duplicates.
5. **Restart Explorer** (task manager, "Restart"): the bars and tray keep working. (US4)
6. **Exit**: end `jtb-app.exe`. Explorer's tray shows the icons again. Then kill it and run `9-restore-explorer.cmd`: same result.

`jtb.log` carries a line for each takeover, release, icon add or drop and each Start click.
