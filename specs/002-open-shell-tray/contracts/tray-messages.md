# Contract: what the tray window answers

Window class `Shell_TrayWnd`, owned by our process, topmost, not visible.

| Message | Action |
|---|---|
| `WM_COPYDATA`, `dwData` 1 | Decode the notify packet, queue a `TrayChange`, return 1 when handled. |
| `WM_COPYDATA`, `dwData` 3 | Answer the icon rectangle query for an icon we show, else forward. |
| `WM_COPYDATA`, `dwData` 0 | Forward to Explorer's `Shell_TrayWnd` (AppBar calls, including ours). |
| `WM_COMMAND`, `WM_ACTIVATEAPP`, messages >= `WM_USER` | Forward to Explorer's window. |
| anything else | `DefWindowProc`. |

Lifecycle: create after the bars are registered; broadcast `TaskbarCreated` once ready; on exit destroy the window, raise Explorer's `Shell_TrayWnd`, broadcast `TaskbarCreated` again. `--restore-explorer` does the last two steps without needing our window.

Clicks on a shown icon: send the app's `callback_message` to its owner window with `wParam` = icon id and `lParam` = the mouse message (`WM_LBUTTONUP`, `WM_RBUTTONUP`, `WM_LBUTTONDBLCLK`); for version 4 icons the id and cursor position are packed as that version defines. Exact packing is read from the Windows docs at implementation time.
