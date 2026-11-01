# Research: Open-Shell coexistence and tray

Sources: Open-Shell-Menu master branch (`Src/StartMenu/StartMenuDLL/StartButton.cpp`, `StartMenuDLL.cpp`, `Src/StartMenu/StartMenu.cpp`), Cairo ManagedShell `TrayService.cs`. Copies were read from the raw files on GitHub; the installed Open-Shell version on this machine was not compared with master.

## R1: Where does Open-Shell's Start button live?

Finding (read in source): the button is a `WS_POPUP`, layered window owned by Explorer's `Shell_TrayWnd`, created inside Explorer by `StartMenuDLL`. `UpdateStartButtonPosition` follows the Explorer taskbar: it hides the button whenever the taskbar is auto-hidden and has slid off the monitor (`rcTask.top > monitor.bottom - 5` for a bottom taskbar), and shows it while the taskbar is visible.

Consequence: with the Explorer taskbar auto-hidden (what `hide_explorer_taskbar` does today), the Open-Shell button is hidden with it and only appears while the pointer pulls the taskbar in. The inset idea in the spec (leave room for Open-Shell's button on our bar) does not work. The Win key does not depend on the taskbar position (not read in full; to check on Windows).

Decision: the bar draws its own Start button that asks Open-Shell to open its menu. Open-Shell stays in charge of the menu itself.

Alternatives: keep the inset and show Explorer's taskbar partly (defeats the point); draw our own menu (out of scope).

## R2: How do we ask Open-Shell to open the menu?

Finding (read in `StartMenu.cpp`): `StartMenu.exe -toggle` and `-open` are command line switches. When an instance is already running, the new process finds Explorer's taskbar by process id (`FindTaskBar(process)` matches the owner process of `Shell_TrayWnd`, so a `Shell_TrayWnd` of ours is not picked) and posts a registered message to it (`-toggle`), or posts `WM_OPEN` to its hook window (`-open`).

Decision: the Start button runs `StartMenu.exe -toggle` from a configurable path (default `C:\Program Files\Open-Shell\StartMenu.exe`, it exists on this machine).

Unverified: where the menu opens when the Explorer taskbar is hidden. To check on Windows; if it opens at the wrong place, try `-open`, then fall back to sending the Win key.

## R3: How do apps find the tray?

Finding (read in Cairo `TrayService.cs`): apps `FindWindow("Shell_TrayWnd")` and send `WM_COPYDATA`. `dwData` 0 is an AppBar message (`SHAppBarMessage`), 1 is a notify icon packet (add, modify, delete in `dwMessage`), 3 is an icon identifier query. Cairo registers its own window of that class, topmost, hidden, and forwards what it does not handle to Explorer's window, so AppBar calls from all apps (and our own bars) still reach Explorer. It broadcasts `TaskbarCreated` so apps re-send their icons. Cairo creates a `TrayNotifyWnd` as well.

Decision: follow Cairo. Handle `dwData == 1` and `3`, forward `0` and everything else.

Risk found while reading: once our window is the topmost `Shell_TrayWnd`, our own `SHAppBarMessage` calls land in it too. Forwarding must be correct from the first build or the bars lose their work-area reservation.

## R4: Restoring Explorer's tray

Finding (Cairo): on shutdown it puts Explorer's `Shell_TrayWnd` back on top and broadcasts `TaskbarCreated` again so apps add their icons to Explorer.

Decision: same, wired into the existing restore path (exit, panic, `--restore-explorer`). The restore path must not depend on our window still existing: `--restore-explorer` only has to broadcast `TaskbarCreated` and raise Explorer's window.

## R5: System icons

Finding: Cairo shows the icons that apps add through the protocol above. Volume, network, battery and similar are Explorer-owned and use private interfaces (not read here). Not promised in this feature (spec Assumptions).

## R6: Windows version

Not checked on this machine. Cairo's approach is reported to work on Windows 10 and 11; unverified with this app.
