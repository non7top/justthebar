use std::sync::Arc;

use crate::icon::Icon;

/// An icon is identified by the window that owns it and the id the app gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrayKey {
    pub owner: isize,
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayIcon {
    pub key: TrayKey,
    pub tooltip: String,
    pub icon: Option<Arc<Icon>>,
    /// Message sent to the owner window when the icon is clicked.
    pub callback_message: u32,
    /// 0 for the legacy callback format, 4 for `NOTIFYICON_VERSION_4`.
    pub version: u32,
    pub hidden: bool,
}

const WM_CONTEXTMENU: u32 = 0x007B;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_MOUSEHOVER: u32 = 0x02A1;
const WM_MOUSELEAVE: u32 = 0x02A3;
const NIN_SELECT: u32 = 0x0400;
const NIN_POPUPOPEN: u32 = 0x0406;
const NIN_POPUPCLOSE: u32 = 0x0407;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Middle,
    Right,
}

/// What the mouse did on an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mouse {
    Enter,
    Leave,
    Move,
    /// A press; `double` when it came within the double click time of the last one.
    Down {
        side: Side,
        double: bool,
    },
    Up(Side),
}

/// The down, up and double click messages of a button.
fn button_messages(side: Side) -> (u32, u32, u32) {
    match side {
        Side::Left => (0x0201, 0x0202, 0x0203),
        Side::Right => (0x0204, 0x0205, 0x0206),
        Side::Middle => (0x0207, 0x0208, 0x0209),
    }
}

impl TrayIcon {
    /// The messages the owner gets for a mouse event, in order. Icons of version 3 and up also
    /// get the select and context menu notifications that Explorer sends them, and version 4
    /// icons get the popup open and close ones.
    pub fn mouse_messages(&self, event: Mouse) -> Vec<u32> {
        let v3 = self.version >= 3;
        match event {
            Mouse::Enter if self.version > 3 => vec![WM_MOUSEHOVER, NIN_POPUPOPEN],
            Mouse::Enter => vec![WM_MOUSEHOVER],
            Mouse::Leave if self.version > 3 => vec![WM_MOUSELEAVE, NIN_POPUPCLOSE],
            Mouse::Leave => vec![WM_MOUSELEAVE],
            Mouse::Move => vec![WM_MOUSEMOVE],
            Mouse::Down { side, double } => {
                let (down, _, dbl) = button_messages(side);
                vec![if double { dbl } else { down }]
            }
            Mouse::Up(Side::Left) if v3 => vec![button_messages(Side::Left).1, NIN_SELECT],
            Mouse::Up(Side::Right) if v3 => vec![button_messages(Side::Right).1, WM_CONTEXTMENU],
            Mouse::Up(side) => vec![button_messages(side).1],
        }
    }

    /// The `wParam` and `lParam` that carry `message` to the owner. `mouse` is the cursor in
    /// screen coordinates, x in the low word. Version 4 packs the cursor into `wParam` and the
    /// icon id into the high word of `lParam`; older icons get the id in `wParam`.
    pub fn callback_params(&self, message: u32, mouse: u32) -> (usize, isize) {
        if self.version > 3 {
            (mouse as usize, (message | (self.key.id << 16)) as isize)
        } else {
            (self.key.id as usize, message as isize)
        }
    }
}

/// The fields of one notify packet. `None` means the packet did not carry that field, so the
/// stored value stays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayUpdate {
    pub key: TrayKey,
    pub tooltip: Option<String>,
    pub icon: Option<Arc<Icon>>,
    pub callback_message: Option<u32>,
    pub hidden: Option<bool>,
}

impl TrayUpdate {
    pub fn new(key: TrayKey) -> TrayUpdate {
        TrayUpdate {
            key,
            tooltip: None,
            icon: None,
            callback_message: None,
            hidden: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayChange {
    /// `NIM_ADD` and `NIM_MODIFY` alike: creates the icon if unknown, else updates it. Apps
    /// resend adds after Explorer restarts, and some modify an icon they never added here.
    Set(TrayUpdate),
    Delete(TrayKey),
    SetVersion(TrayKey, u32),
}

/// The notification area icons in order of arrival.
#[derive(Debug, Default)]
pub struct TrayIcons {
    items: Vec<TrayIcon>,
    /// Icons an app deleted and has not added again.
    deleted: Vec<TrayKey>,
}

impl TrayIcons {
    /// True while `key` was deleted by its app and not added again: a modify for it is stale.
    pub fn was_deleted(&self, key: TrayKey) -> bool {
        self.deleted.contains(&key)
    }

    /// Returns true when something that is drawn may have changed.
    pub fn apply(&mut self, change: TrayChange) -> bool {
        match change {
            TrayChange::Set(update) => {
                self.deleted.retain(|k| *k != update.key);
                let at = match self.items.iter().position(|i| i.key == update.key) {
                    Some(at) => at,
                    None => {
                        self.items.push(TrayIcon {
                            key: update.key,
                            tooltip: String::new(),
                            icon: None,
                            callback_message: 0,
                            version: 0,
                            hidden: false,
                        });
                        self.items.len() - 1
                    }
                };
                let item = &mut self.items[at];
                if let Some(tooltip) = update.tooltip {
                    item.tooltip = tooltip;
                }
                if let Some(icon) = update.icon {
                    item.icon = Some(icon);
                }
                if let Some(message) = update.callback_message {
                    item.callback_message = message;
                }
                if let Some(hidden) = update.hidden {
                    item.hidden = hidden;
                }
                true
            }
            TrayChange::Delete(key) => {
                if !self.deleted.contains(&key) {
                    self.deleted.push(key);
                }
                let before = self.items.len();
                self.items.retain(|i| i.key != key);
                self.items.len() != before
            }
            TrayChange::SetVersion(key, version) => {
                match self.items.iter_mut().find(|i| i.key == key) {
                    Some(item) => {
                        item.version = version;
                        true
                    }
                    None => false,
                }
            }
        }
    }

    /// Removes the icons whose owner window is gone; apps that crash never send a delete.
    /// Returns true when any icon was removed.
    pub fn drop_dead(&mut self, alive: impl Fn(isize) -> bool) -> bool {
        let before = self.items.len();
        self.items.retain(|i| alive(i.key.owner));
        self.deleted.retain(|k| alive(k.owner));
        self.items.len() != before
    }

    pub fn get(&self, key: TrayKey) -> Option<&TrayIcon> {
        self.items.iter().find(|i| i.key == key)
    }

    pub fn visible(&self) -> impl Iterator<Item = &TrayIcon> {
        self.items.iter().filter(|i| !i.hidden)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(owner: isize, id: u32) -> TrayKey {
        TrayKey { owner, id }
    }

    fn named(key: TrayKey, tooltip: &str) -> TrayChange {
        TrayChange::Set(TrayUpdate {
            tooltip: Some(tooltip.to_string()),
            ..TrayUpdate::new(key)
        })
    }

    fn icon(id: u64) -> Arc<Icon> {
        Arc::new(Icon {
            id,
            width: 1,
            height: 1,
            bgra: vec![0; 4],
        })
    }

    fn tooltips(icons: &TrayIcons) -> Vec<&str> {
        icons.visible().map(|i| i.tooltip.as_str()).collect()
    }

    #[test]
    fn icons_keep_the_order_they_arrived_in() {
        let mut icons = TrayIcons::default();
        icons.apply(named(key(1, 1), "one"));
        icons.apply(named(key(2, 1), "two"));
        icons.apply(named(key(1, 2), "three"));
        assert_eq!(tooltips(&icons), vec!["one", "two", "three"]);
    }

    #[test]
    fn the_same_key_is_one_icon_and_keeps_its_place() {
        let mut icons = TrayIcons::default();
        icons.apply(named(key(1, 1), "one"));
        icons.apply(named(key(2, 1), "two"));
        icons.apply(named(key(1, 1), "uno"));
        assert_eq!(icons.len(), 2);
        assert_eq!(tooltips(&icons), vec!["uno", "two"]);
    }

    #[test]
    fn a_packet_only_changes_the_fields_it_carries() {
        let mut icons = TrayIcons::default();
        icons.apply(TrayChange::Set(TrayUpdate {
            tooltip: Some("tip".into()),
            icon: Some(icon(7)),
            callback_message: Some(0x401),
            ..TrayUpdate::new(key(1, 1))
        }));
        icons.apply(TrayChange::Set(TrayUpdate {
            hidden: Some(true),
            ..TrayUpdate::new(key(1, 1))
        }));
        let item = icons.get(key(1, 1)).unwrap();
        assert_eq!(item.tooltip, "tip");
        assert_eq!(item.icon.as_ref().map(|i| i.id), Some(7));
        assert_eq!(item.callback_message, 0x401);
        assert!(item.hidden);
        assert_eq!(icons.visible().count(), 0);
    }

    #[test]
    fn a_modify_of_an_unknown_icon_adds_it() {
        let mut icons = TrayIcons::default();
        assert!(icons.apply(named(key(5, 9), "late")));
        assert_eq!(tooltips(&icons), vec!["late"]);
    }

    #[test]
    fn delete_removes_only_that_icon_and_reports_change() {
        let mut icons = TrayIcons::default();
        icons.apply(named(key(1, 1), "one"));
        icons.apply(named(key(1, 2), "two"));
        assert!(icons.apply(TrayChange::Delete(key(1, 1))));
        assert!(!icons.apply(TrayChange::Delete(key(1, 1))));
        assert_eq!(tooltips(&icons), vec!["two"]);
    }

    #[test]
    fn a_deleted_icon_is_stale_until_added_again() {
        let mut icons = TrayIcons::default();
        assert!(!icons.was_deleted(key(1, 1)));
        icons.apply(TrayChange::Delete(key(1, 1)));
        assert!(icons.was_deleted(key(1, 1)));
        assert!(!icons.was_deleted(key(1, 2)));
        icons.apply(named(key(1, 1), "one"));
        assert!(!icons.was_deleted(key(1, 1)));
    }

    #[test]
    fn version_applies_to_known_icons_only() {
        let mut icons = TrayIcons::default();
        icons.apply(named(key(1, 1), "one"));
        assert!(icons.apply(TrayChange::SetVersion(key(1, 1), 4)));
        assert_eq!(icons.get(key(1, 1)).unwrap().version, 4);
        assert!(!icons.apply(TrayChange::SetVersion(key(2, 2), 4)));
        assert_eq!(icons.len(), 1);
    }

    #[test]
    fn icons_of_dead_owners_are_dropped() {
        let mut icons = TrayIcons::default();
        icons.apply(named(key(1, 1), "one"));
        icons.apply(named(key(2, 1), "two"));
        icons.apply(named(key(2, 2), "three"));
        assert!(icons.drop_dead(|owner| owner == 1));
        assert_eq!(tooltips(&icons), vec!["one"]);
        assert!(!icons.drop_dead(|owner| owner == 1));
    }

    fn item(version: u32) -> TrayIcon {
        TrayIcon {
            key: key(1, 7),
            tooltip: String::new(),
            icon: None,
            callback_message: 0x401,
            version,
            hidden: false,
        }
    }

    #[test]
    fn old_icons_get_the_id_in_wparam_and_version_4_packs_the_cursor() {
        assert_eq!(item(0).callback_params(0x202, 0x0064_00C8), (7, 0x202));
        assert_eq!(
            item(4).callback_params(0x202, 0x0064_00C8),
            (0x0064_00C8, 0x0007_0202)
        );
    }

    #[test]
    fn a_left_click_ends_with_select_on_version_3_and_up() {
        assert_eq!(item(0).mouse_messages(Mouse::Up(Side::Left)), vec![0x202]);
        assert_eq!(
            item(3).mouse_messages(Mouse::Up(Side::Left)),
            vec![0x202, 0x400]
        );
        assert_eq!(
            item(4).mouse_messages(Mouse::Up(Side::Right)),
            vec![0x205, 0x7B]
        );
        assert_eq!(item(1).mouse_messages(Mouse::Up(Side::Middle)), vec![0x208]);
    }

    #[test]
    fn a_press_is_a_double_click_message_when_it_came_quickly() {
        let down = |side, double| item(0).mouse_messages(Mouse::Down { side, double });
        assert_eq!(down(Side::Left, false), vec![0x201]);
        assert_eq!(down(Side::Left, true), vec![0x203]);
        assert_eq!(down(Side::Right, true), vec![0x206]);
        assert_eq!(down(Side::Middle, false), vec![0x207]);
    }

    #[test]
    fn hover_messages_add_the_popup_pair_on_version_4_only() {
        assert_eq!(item(3).mouse_messages(Mouse::Enter), vec![0x2A1]);
        assert_eq!(item(4).mouse_messages(Mouse::Enter), vec![0x2A1, 0x406]);
        assert_eq!(item(4).mouse_messages(Mouse::Leave), vec![0x2A3, 0x407]);
        assert_eq!(item(4).mouse_messages(Mouse::Move), vec![0x200]);
    }
}
