# Feature Specification: Task grouping, pins and drag and drop

**Created**: 2026-10-31
**Status**: Implemented, waiting for testing on Windows
**Input**: accordion grouping of task buttons, same-app and custom groups, pins that launch and group, drag and drop, groups that mix different executables.

## User Scenarios

### US1 - Windows of one app collapse into an accordion (P1)

Windows of the same application (same executable) share one group. A group of two or more windows shows a header button and the member windows as child buttons under it. Clicking the header folds or unfolds the group (a pin with no windows launches instead). Middle click on the header launches a new instance of the app. A group of one window, with no pin, is a plain task button.

**Independent test**: open three Notepad windows; the bar shows one header "notepad (3)" and three children. A click on the header folds them; the header keeps the flash, title-change and active marks of its windows.

**Acceptance**:
1. Given 3 windows of one exe, when the bar draws, then one header and 3 children are shown, in window order.
2. Given a folded group, when a member window flashes, then the header shows the flash color.
3. Given the header, when it is middle clicked, then a new instance of the exe starts.
4. Given the header, when it is left clicked, then the group folds, and unfolds on the next click.

### US2 - Pins launch apps and own their group (P2)

A pin is a header that exists without windows. Middle clicking it launches the app (a plain click launches too while it has no windows); windows of that app then group under the pin. Pins come from the config, and from dropping an app onto the bar.

**Independent test**: with a pin for Notepad and no Notepad open, the bar shows the pin, a click starts Notepad, and its window appears under the pin.

### US3 - Custom groups (P3)

The config can define named groups of executables. Windows of any listed exe form one group, even though they are different apps. A pin whose exe is in a group claims the whole group (for example PuTTY Portable's launcher plus the putty.exe windows it starts).

**Independent test**: group "putty" with `PuTTYPortable.exe` and `putty.exe`; windows of both appear under one header.

### US4 - Drag and drop (P3)

Dragging a task button, header or pin moves it to another position on the bar. Dropping an exe or a shortcut from Explorer onto the bar pins it. Any pin, from the config or added from the bar, can be removed from the header's context menu; a config pin is hidden through `jtb.state.json`, so `jtb.json` stays untouched.

**Independent test**: drag the second header before the first; the order stays after a restart.

## Requirements

- **FR-001**: The model decides groups and rows in `jtb-core`, tested on Linux.
- **FR-002**: Group key is the lowercase exe path, or the custom group the exe belongs to. UWP windows hosted by ApplicationFrameHost are never grouped together.
- **FR-003**: Mouse actions are a table (button, target) to action (`activate`, `launch`, `toggle_fold`, `none`), defaults as in US1, overridable in config.
- **FR-004**: `grouping.mode` is `accordion` (default) or `off`; `off` restores plain buttons.
- **FR-005**: Fold state, order, drag-added pins and hidden config pins live in `jtb.state.json` next to the exe; `jtb.json` is never rewritten by the app.
- **FR-006**: A header stands for its group: active if any member is, flashing if any flashes, title-changed if any changed and none is active, dimmed when all members are minimized or the pin has no windows.
- **FR-007**: Pins and custom groups are validated at load; a bad file keeps the last good config.

## Out of scope

AUMID based grouping, thumbnails on headers (M4), jump lists, per-group icons overrides.

## Assumptions

- Exe path is read with `QueryFullProcessImageNameW` from the window's process; if it cannot be read the window stays ungrouped.
- Dropped shortcuts are resolved to their target exe.
