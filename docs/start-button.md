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
  "size": 54
}
```

Every key but `enabled` is optional and the values above are the defaults.

Menu position: Open-Shell opens its menu next to its own button, a popup window owned by
Explorer's hidden taskbar. On a click the bar moves that button under its Start button, then runs
the command. The log line `start: Open-Shell button asked to move to ..., now at ...` shows where
the button ended up. Which side of the button the menu grows to still follows the edge of
Explorer's taskbar (the bottom), so on a side bar the menu can overlap the bar.

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
