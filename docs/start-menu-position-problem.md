# Problem: make Open-Shell's Start menu open next to a custom taskbar's Start button

## Setup

- Windows 10/11 desktop. A custom taskbar (Rust, Win32, "justthebar") runs as AppBars on several
  screen edges, while explorer.exe keeps running with its own taskbar set to auto-hide (so
  Explorer's Shell_TrayWnd is a hidden 48 px strip at the bottom, 0,1032,1920,1080 on a
  1920x1080 screen).
- Our process creates its own top-most "Shell_TrayWnd" window (the way Cairo Desktop does) so apps
  send us tray icons (WM_COPYDATA). Everything we don't handle is forwarded to Explorer's.
- Open-Shell 4.4.198 (classic Start menu) is installed and owns the menu. Our bar draws its own
  Start button and on click runs `StartMenu.exe -toggle`.
- Goal: the menu opens next to OUR Start button. Example: bar on the right edge
  (1720,0,1920,1080), button at 1793,4,1847,58 -> the menu should open left of the bar, near the
  top.

## What happens

The menu opens at the bottom-left of the screen (or partly off-screen when Open-Shell's "Align
start menu to working area" is off). That is where Open-Shell's own button sits.

## What we tried

1. On click, move Open-Shell's own button (window class `OpenShell.CStartButton`, owned by
   Explorer's Shell_TrayWnd) with SetWindowPos to our button's rectangle. It works, but see 4.
2. Answer ABM_GETTASKBARPOS (WM_COPYDATA dwData 0, shared memory via SHLockShared) from our
   Shell_TrayWnd with the clicked bar's edge and rectangle. We answer it, but see 3.
3. Tried stretching Explorer's own hidden taskbar window to our bar's rectangle. Explorer
   immediately puts it back (reverted).
4. Latest build (untested): re-apply the button move every 10 ms for 700 ms after the click.

## What we measured

Sources: our own log, and Open-Shell's menu log (enabled with
`HKCU\Software\OpenShell\StartMenu\Settings` `LogCategories`=1, written to
`%LOCALAPPDATA%\OpenShell\StartMenuLog.txt`, overwritten on every menu open).

- The menu runs inside explorer.exe: Open-Shell's log has PID=11048, and Explorer's Shell_TrayWnd
  belongs to PID 11048.
- Open-Shell's log shows "Taskbar Rect: 0,1032,1920,1080" (Explorer's strip), "Main Rect:
  0,0,1920,1080", "Start Rect: 6,1076,60,1130". None of our values (edge RIGHT, 1720,0,1920,1080
  and 1793,4,1847,58) appear.
- Our ABM_GETTASKBARPOS handler is reached only by another process (6488), never by 11048. Inferred,
  not proven: GetTaskbarPosition's SHAppBarMessage(ABM_GETTASKBARPOS), called from inside
  explorer.exe, is answered by Explorer's own tray and never reaches our window.
- About 200 ms after the click Open-Shell moves its button back (sampled every 60 ms): 1793,4
  (ours) -> 933,1078 or 6,1076 -> 6,1030. The menu reads the button rectangle around the 6,1076
  sample, which matches the "Start Rect" in its log.

## Relevant Open-Shell code

Files: `Src/StartMenu/StartMenuDLL/StartMenuDLL.cpp` and `MenuContainer.cpp`.

- `GetTaskbarPosition()`: for the cached main taskbar `g_TaskBar`, it calls
  `SHAppBarMessage(ABM_GETTASKBARPOS)` and clamps the result to `GetWindowRect(taskBar)`.
- `ToggleStartMenu`: edge LEFT/RIGHT/TOP -> CONTAINER_TOP (menu starts at the button's height);
  edge BOTTOM -> menu anchored to the bottom of the limits. `s_StartRect = GetWindowRect(button)`.
  CONTAINER_LEFT if `start.left + start.right < limits.left + limits.right`.
- There is an in-process path via a StartMenuMsg window message (lParam = pointer to
  `StartMenuParams` with edge, rects, button) that skips all this, but a pointer only works inside
  explorer.exe.

## Constraints

- No injecting into explorer.exe, no patching Open-Shell (it is an unmodified install).
- The bar must keep working with Explorer running; Explorer's taskbar stays auto-hidden.

## Questions

1. Is there a supported or known-working way to make an in-explorer SHAppBarMessage
   (ABM_GETTASKBARPOS) see a different taskbar position, for example by changing Explorer's
   taskbar edge/state through a documented route?
2. How do other third-party taskbars (Cairo, RetroBar, 7+ Taskbar Tweaker, ExplorerPatcher) get
   Open-Shell or the stock Start menu to open at their Start button?
3. Any other idea that makes Open-Shell compute the menu position from our button?
