Here is a modular, multi-phase plan designed for an AI agent to extract the system logic from Cairo Desktop and implement it as a custom, high-performance taskbar engine in Rust.

Phase 1: Architecture Mapping & OS Plumbing

• Analyze Cairo Source: Direct the agent to read Cairo's ManagedShell repository, specifically targeting how it hooks SHAppBarMessage (for reserving screen real estate) and RegisterShellHookWindow (for catching window open/close events).
• Rust Baseline Implementation: Translate those foundational Win32 patterns into a pure Rust architecture using the windows-rs crate.
• Multi-Monitor Spawning: Implement logic to detect all active display outputs (EnumDisplayMonitors) and launch a separate, independent taskbar process thread or container window anchored to each individual screen.

Phase 2: Core Task Management & Dynamic Grouping

• Window Tracking Engine: Build a centralized background thread that maintains a real-time vector (Vec<HWND>) of active applications filtering out hidden background processes.
• Dynamic Grouping Logic: Create an internal data model that indexes processes by their unique identifier (AppUserModelID).
• Interactive Toggling: Implement a messaging pipeline so that clicking a button triggers a state update, instantly rendering the tasks either as individual buttons with text labels (Windows 7 ungrouped) or combined squares (Windows 7 Superbar grouped).

Phase 3: Integration & Custom Configuration

• Open-Shell & Tray Hook: Map the system tray container to align with Open-Shell's positioning guidelines. Use inter-process communication (IPC) or custom tray message relays to ensure Open-Shell handles the Start menu behavior smoothly while your custom tray handles notifications.
• JSON Settings System: Build a central configuration module (config.json) that manages layouts, grouping rules, and monitor assignments, supporting on-the-fly reloads when the file updates.

Phase 4: Design Abstraction for Future Aero Glass

• UI Separation Layer: Abstract the visual engine away from the system logic. Use a lightweight, clean rendering framework (like egui or slint) that handles layout calculations but renders elements as transparent boxes.
• Styling Extension Bridge: Design the layout engine to leave gaps/frames exposed so that in a future phase, a native composition pipeline (SetWindowCompositionAttribute) or an external DWM companion library can draw frosted glass texture grids directly into the target areas.
Would you like me to generate the exact config.json schema for the agent to use, or the initial Cargo.toml dependencies manifest to get Phase 1 started?
