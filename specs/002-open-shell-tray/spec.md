# Feature Specification: Open-Shell coexistence and system tray

**Feature Branch**: `feat/taskbar-engine` (spec directory `002-open-shell-tray`)
**Created**: 2026-11-01
**Status**: Draft
**Input**: User description: "Open-Shell and system tray integration: the bar coexists with Open-Shell (Start button and Win key keep working, optional inset at the start edge so the bar does not cover the Start button) and hosts the notification area (tray icons) in place of the hidden Explorer taskbar, using Cairo's approach of our own Shell_TrayWnd receiving WM_COPYDATA while Explorer keeps running."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Start button on the bar opens the Open-Shell menu (Priority: P1)

The user runs Open-Shell for the Start menu. Open-Shell's own button follows the Explorer taskbar and hides with it while that taskbar is auto-hidden, so the bar shows its own Start button. Clicking it opens the Open-Shell menu; the Win key keeps working.

**Why this priority**: Start is how the user launches everything, and without a visible button the user depends on the Win key alone. It needs no tray work.

**Independent Test**: Run the app with Open-Shell installed and the Start button enabled in `jtb.json`. Click the Start button, then press the Win key.

**Acceptance Scenarios**:

1. **Given** the bar is running with the Explorer taskbar hidden, **When** the user presses the Win key, **Then** the Open-Shell Start menu opens.
2. **Given** `start.enabled` is true, **When** the bar draws, **Then** a Start button is the first button at the start edge of each bar with task buttons.
3. **Given** the Start button, **When** the user clicks it, **Then** the Open-Shell menu opens, and clicking again closes it.
4. **Given** the command in `jtb.json` is wrong, **When** the user clicks, **Then** nothing breaks and the failure is in `jtb.log`.
5. **Given** the config changes while the app runs, **When** the file is saved, **Then** the button appears or disappears without a restart.

---

### User Story 2 - Tray icons show on the bar (Priority: P2)

The notification area lives on the bar: icons of running apps (volume, network, chat clients, sync tools) appear in a tray area next to the clock, with their tooltips.

**Why this priority**: Without a tray the user must unhide the Explorer taskbar to reach any icon, which defeats the replacement. It depends on the app being safely registered as the taskbar (US3's restore rules).

**Independent Test**: Start an app that adds a tray icon (for example the volume or a sync client); its icon appears on the bar within a second, and removing the icon from the app removes it from the bar.

**Acceptance Scenarios**:

1. **Given** the app is running, **When** another app adds a tray icon, **Then** the icon shows in the tray area with its image.
2. **Given** an icon is shown, **When** the owning app changes its image or tooltip, **Then** the bar updates it.
3. **Given** an icon is shown, **When** the owning app removes it or exits, **Then** the icon disappears.
4. **Given** apps were already running before the bar started, **When** the bar starts, **Then** their icons appear without the user restarting them.
5. **Given** several bars, **When** tray icons exist, **Then** only the bar configured to host the tray shows them.

---

### User Story 3 - Tray icon clicks reach the owning app (Priority: P2)

Left, right and double clicks on a tray icon do what they do in Explorer: open the app's window, show its menu, show its balloon notifications.

**Why this priority**: Icons that cannot be clicked are decoration. It is the same milestone as US2 and tested together.

**Independent Test**: Right click a tray icon of any app; the app's own context menu appears at the cursor.

**Acceptance Scenarios**:

1. **Given** a visible icon, **When** the user right clicks it, **Then** the owning app's context menu opens at the cursor.
2. **Given** a visible icon, **When** the user double clicks it, **Then** the owning app receives a double click and acts on it.
3. **Given** an app shows a balloon notification for its icon, **When** the notification fires, **Then** it is shown or, at minimum, does not block the app.

---

### User Story 4 - Explorer is given back whenever the app stops (Priority: P1)

Taking over the tray must never leave the desktop without a working taskbar: on exit, crash or `--restore-explorer`, tray ownership returns to Explorer and the apps re-add their icons to it.

**Why this priority**: This is a project rule (constitution principle II) and the riskiest part of this feature.

**Independent Test**: Run with the tray, kill the process from the task manager, run `--restore-explorer`; Explorer's taskbar shows its tray icons again.

**Acceptance Scenarios**:

1. **Given** the tray is hosted, **When** the app exits normally, **Then** Explorer's taskbar tray works again and apps' icons are back on it.
2. **Given** the app was killed, **When** `--restore-explorer` runs, **Then** the same holds.
3. **Given** Explorer restarts while the app runs, **When** its taskbar is recreated, **Then** the bar keeps working and the tray recovers.

---

### Edge Cases

- A tray app sends its icon before the bar is ready, or Explorer restarts and every app re-sends: icons must not duplicate.
- An app's process dies without removing its icon: the stale icon is dropped within a few seconds.
- Icons with no image, a huge image, or a very long tooltip.
- Hidden or "overflow" icons (Windows 10 and 11 let the user hide icons): out of scope for the first version, all icons show.
- The bar hosting the tray is on the side edge: icons stack vertically in the available width.
- High DPI and mixed-DPI monitors: icons are sharp at the bar's scale.
- The system icons (volume, network, battery, clock flyout, action center) belong to Explorer and are not ours to draw: see Assumptions.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each bar with task buttons MUST be able to show a Start button that runs a configurable command (default: Open-Shell's `StartMenu.exe -toggle`), switched and applied live from `jtb.json`.
- **FR-002**: Running the bar MUST NOT stop the Win key from opening the Start menu, and a failing Start command MUST NOT affect the bar.
- **FR-003**: The app MUST host a tray area on exactly one configured bar and show each notification icon (image, tooltip) added by other apps.
- **FR-004**: The tray MUST follow icon add, modify and delete from apps, and MUST show icons of apps that were already running at start.
- **FR-005**: Clicks on a tray icon MUST be delivered to the owning app so it can act (window, menu, balloon).
- **FR-006**: When the app stops for any reason, tray ownership MUST return to Explorer and apps MUST be told to re-add their icons; `--restore-explorer` MUST do the same after a crash.
- **FR-007**: Tray hosting MUST be switchable in `jtb.json` (default off until verified on the user's machine), and a bad value keeps the last good config.
- **FR-008**: Tray and Open-Shell state changes that depend on the live system (ownership taken or released, icons added or dropped, Start button found or not) MUST write a line to `jtb.log`.
- **FR-009**: Decisions about which icons exist and their order MUST live in `jtb-core` and be unit tested without Windows.

### Key Entities

- **Tray icon**: owner window, icon id, image, tooltip, state (visible, removed), the time it was last seen alive.
- **Tray host**: the bar and area that shows icons, and the record of who currently owns the taskbar role.
- **Start button**: a button at the start edge of a bar with a label and the command it runs.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the bar running, the Win key opens the Start menu in 10 out of 10 attempts.
- **SC-002**: A new tray icon appears on the bar within 1 second of the owning app adding it.
- **SC-003**: After a normal exit or a kill followed by `--restore-explorer`, 100% of the icons present before the bar started show again on Explorer's tray.
- **SC-004**: A day of normal use (apps opened and closed, one Explorer restart, one monitor change) ends with no duplicate and no stale icon on the bar.
- **SC-005**: The user does not need to unhide the Explorer taskbar for any daily task except the system icons named in Assumptions.

## Assumptions

- Open-Shell stays in charge of the Start menu. Its own button hides with the auto-hidden Explorer taskbar (read in Open-Shell's source, see research.md), so the bar draws a button that asks Open-Shell to open its menu. Where the menu opens with the Explorer taskbar hidden has not been checked on this machine.
- Taking the tray role works with Explorer running: the app owns a taskbar window that apps find, and forwards what it does not handle to Explorer. This follows the approach of Cairo Desktop's ManagedShell and has not been tried with this app on Windows.
- Windows system icons (volume, network, battery, input language) and their flyouts are handled by Explorer through private interfaces. Whether they appear on our bar is not promised; if they do not, they stay reachable through Explorer's own taskbar until a later feature.
- The clock already on the bar stays as it is; flyouts for it are out of scope.
- Windows 10 and 11 desktop only, as for the rest of the project.
- Balloon notifications may be dropped silently in the first version.
