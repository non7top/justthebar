# Start button

The bar can draw a Start button as its first button. A click runs a command, by default
Open-Shell's `StartMenu.exe -toggle`, so Open-Shell stays in charge of the menu. Open-Shell's own
button is hidden together with the auto-hidden Explorer taskbar, which is why the bar has its own.

## Turn it on

In `jtb.json`:

```json
"start": {
  "enabled": true,
  "command": "C:\\Program Files\\Open-Shell\\StartMenu.exe",
  "args": ["-toggle"],
  "label": "Start",
  "image": "start-button.png",
  "size": 54,
  "in_process": true
}
```

Every key but `enabled` is optional and the values above are the defaults.

## Menu position

Open-Shell works out where to put its menu from Explorer's own (hidden) taskbar, so on a custom bar
the menu lands at the bottom-left or off screen. Two ways around that:

`"in_process": true` (the default). The bar loads Open-Shell's `StartMenuDLL.dll` (next to the
`command`) into its own process, the way `StartMenu.exe -nohook` does, and opens the menu there. It
tells the menu the clicked bar's edge, the bar's and the button's rectangles and the monitor's
work area, through the placement message Open-Shell documents in `MenuContainer.h`. The menu then
opens next to the button on any edge, its submenus get the full work area, and a click elsewhere
closes it because the bar holds the foreground. Explorer's copy of Open-Shell is untouched, so the
Win key keeps working. If the library cannot be loaded or lacks the functions, the log says
`start: in-process menu failed: ...` and the bar uses the default route below. The function names
it looks up belong to Open-Shell 4.4.198; another version may need a change here.

The menu takes its scale from the monitor of the clicked bar (a 1-pixel anchor window is moved
there before each open). That only works because `jtb-app.exe` carries a `supportedOS` manifest
(`crates/jtb-app/jtb-app.manifest`, compiled by `build.rs` with `x86_64-w64-mingw32-windres`):
without it Windows reports version 6.2 and Open-Shell skips its per-monitor DPI code.

Fallback (`"in_process": false`, or when the library cannot be loaded). A click moves Open-Shell's own button under the bar's button,
answers Open-Shell's "where is the taskbar" question with the clicked bar, and posts Open-Shell's
toggle message to Explorer's taskbar (what `StartMenu.exe -toggle` does). Open-Shell asks twice
and the second question sometimes goes to Explorer's taskbar, so the menu opens in the right place
only some of the time, and its submenus are limited to a thin strip.

## Look

Without an image the button is four small squares and the label.

With an image the button shows that art instead: a square at the start of the bar, `size` pixels
wide at 100% scale (never thicker than the bar, centered across it), with the hot
frame faded in over the normal one while the pointer is on it (about 300 ms, like Open-Shell) and
the pressed frame while the mouse button is down.

The image is a PNG strip in Open-Shell's format: equal frames stacked vertically, in the order
normal, hot, pressed. A strip with a header row (the 12-frame strip inside Open-Shell) works too;
its frame list names which frames are normal, hot and pressed, and its animation data is ignored.

## Where to get art

The art is not part of this repository, and the images posted by users have no stated license. Pick
one for your own use:

1. Download a Start button image from the Classic Shell forum, board "Start Buttons":
   <https://www.classicshell.net/forum/viewforum.php?f=18>
   Many posts are a strip of three frames, normal, hot and pressed.
2. Save it as `start-button.png` next to `jtb-app.exe`, or set `start.image` to another file name
   (relative to the exe folder) or to an absolute path.
3. Restart `jtb-app.exe`. The image is read once per run.

If the file is missing or is not a usable strip, the app logs why in `jtb.log` and draws the
square version.
