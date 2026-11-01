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
}

impl TrayIcons {
    /// Returns true when something that is drawn may have changed.
    pub fn apply(&mut self, change: TrayChange) -> bool {
        match change {
            TrayChange::Set(update) => {
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
}
