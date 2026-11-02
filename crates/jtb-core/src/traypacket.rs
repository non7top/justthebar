// Layout and handling follow ManagedShell.WindowsTray (Apache-2.0): SHELLTRAYDATA from
// NativeMethods.Shell32.cs, the add/modify/delete rules from NotificationArea.cs.
use std::sync::Arc;

use crate::icon::Icon;
use crate::tray::{TrayChange, TrayKey, TrayUpdate};

pub const NIM_ADD: u32 = 0;
pub const NIM_MODIFY: u32 = 1;
pub const NIM_DELETE: u32 = 2;
pub const NIM_SETVERSION: u32 = 4;

pub const NIF_MESSAGE: u32 = 0x1;
pub const NIF_ICON: u32 = 0x2;
pub const NIF_TIP: u32 = 0x4;
pub const NIF_STATE: u32 = 0x8;

pub const NIS_HIDDEN: u32 = 0x1;

/// Size of the `SHELLTRAYDATA` the shell sends with a notify icon message.
pub const SHELL_TRAY_DATA_SIZE: usize = 964;

// Offsets inside SHELLTRAYDATA: an 8 byte header, then NOTIFYICONDATA with 32-bit handles.
const MESSAGE: usize = 4;
const HWND: usize = 12;
const ID: usize = 16;
const FLAGS: usize = 20;
const CALLBACK: usize = 24;
const ICON: usize = 28;
const TIP: usize = 32;
const TIP_CHARS: usize = 128;
const STATE: usize = 288;
const STATE_MASK: usize = 292;
const VERSION: usize = 808;
const GUID: usize = 944;

/// What an app asked for with `Shell_NotifyIcon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyPacket {
    /// One of the `NIM_*` values.
    pub message: u32,
    pub hwnd: isize,
    pub id: u32,
    pub flags: u32,
    pub callback_message: u32,
    /// The app's icon handle; valid in any process.
    pub icon_handle: isize,
    pub tip: String,
    pub state: u32,
    pub state_mask: u32,
    pub version: u32,
    pub guid: [u8; 16],
}

fn u32_at(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}

/// Handles travel as 32-bit values that are sign extended in a 64-bit process.
fn handle_at(buf: &[u8], at: usize) -> isize {
    u32_at(buf, at) as i32 as isize
}

/// Reads the `dwData` 1 payload of a `WM_COPYDATA`. Senders with an older, shorter structure
/// leave the missing fields at zero. Returns `None` when even the leading fields are missing.
pub fn parse_notify(data: &[u8]) -> Option<NotifyPacket> {
    if data.len() < TIP {
        return None;
    }
    let mut buf = [0u8; SHELL_TRAY_DATA_SIZE];
    let len = data.len().min(SHELL_TRAY_DATA_SIZE);
    buf[..len].copy_from_slice(&data[..len]);

    let tip_units: Vec<u16> = buf[TIP..TIP + TIP_CHARS * 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .take_while(|c| *c != 0)
        .collect();
    let mut guid = [0u8; 16];
    guid.copy_from_slice(&buf[GUID..GUID + 16]);
    Some(NotifyPacket {
        message: u32_at(&buf, MESSAGE),
        hwnd: handle_at(&buf, HWND),
        id: u32_at(&buf, ID),
        flags: u32_at(&buf, FLAGS),
        callback_message: u32_at(&buf, CALLBACK),
        icon_handle: handle_at(&buf, ICON),
        tip: String::from_utf16_lossy(&tip_units),
        state: u32_at(&buf, STATE),
        state_mask: u32_at(&buf, STATE_MASK),
        version: u32_at(&buf, VERSION),
        guid,
    })
}

impl NotifyPacket {
    pub fn key(&self) -> TrayKey {
        TrayKey {
            owner: self.hwnd,
            id: self.id,
        }
    }

    /// True when an add or modify carries an icon the caller has to turn into pixels first.
    pub fn wants_icon(&self) -> bool {
        matches!(self.message, NIM_ADD | NIM_MODIFY)
            && self.flags & NIF_ICON != 0
            && self.icon_handle != 0
    }

    pub fn is_modify(&self) -> bool {
        self.message == NIM_MODIFY
    }

    /// False for packets that cannot be applied: no owner window.
    pub fn is_valid(&self) -> bool {
        self.hwnd != 0
    }

    /// The model changes this packet stands for. `icon` is the decoded icon when `wants_icon`.
    pub fn changes(&self, icon: Option<Arc<Icon>>) -> Vec<TrayChange> {
        if !self.is_valid() {
            return Vec::new();
        }
        let key = self.key();
        match self.message {
            NIM_ADD | NIM_MODIFY => {
                let mut update = TrayUpdate::new(key);
                if self.flags & NIF_TIP != 0 && !self.tip.is_empty() {
                    update.tooltip = Some(self.tip.clone());
                }
                if self.flags & NIF_ICON != 0 {
                    update.icon = icon;
                }
                if self.flags & NIF_MESSAGE != 0 {
                    update.callback_message = Some(self.callback_message);
                }
                if self.flags & NIF_STATE != 0 && self.state_mask & NIS_HIDDEN != 0 {
                    update.hidden = Some(self.state & NIS_HIDDEN != 0);
                }
                let mut changes = vec![TrayChange::Set(update)];
                if (1..=4).contains(&self.version) {
                    changes.push(TrayChange::SetVersion(key, self.version));
                }
                changes
            }
            NIM_DELETE => vec![TrayChange::Delete(key)],
            NIM_SETVERSION if self.version <= 4 => vec![TrayChange::SetVersion(key, self.version)],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(buf: &mut [u8], at: usize, value: u32) {
        buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn packet(message: u32, hwnd: u32, id: u32, flags: u32) -> Vec<u8> {
        let mut buf = vec![0u8; SHELL_TRAY_DATA_SIZE];
        put(&mut buf, MESSAGE, message);
        put(&mut buf, HWND, hwnd);
        put(&mut buf, ID, id);
        put(&mut buf, FLAGS, flags);
        buf
    }

    fn put_tip(buf: &mut [u8], tip: &str) {
        for (i, unit) in tip.encode_utf16().enumerate() {
            buf[TIP + i * 2..TIP + i * 2 + 2].copy_from_slice(&unit.to_le_bytes());
        }
    }

    #[test]
    fn fields_are_read_from_their_offsets() {
        let mut buf = packet(
            NIM_ADD,
            0x1234,
            7,
            NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_STATE,
        );
        put(&mut buf, CALLBACK, 0x401);
        put(&mut buf, ICON, 0x55);
        put(&mut buf, STATE, 1);
        put(&mut buf, STATE_MASK, 1);
        put(&mut buf, VERSION, 4);
        put_tip(&mut buf, "Volume");
        buf[GUID] = 9;
        let p = parse_notify(&buf).unwrap();
        assert_eq!(p.message, NIM_ADD);
        assert_eq!(p.hwnd, 0x1234);
        assert_eq!(p.id, 7);
        assert_eq!(p.callback_message, 0x401);
        assert_eq!(p.icon_handle, 0x55);
        assert_eq!(p.tip, "Volume");
        assert_eq!((p.state, p.state_mask, p.version), (1, 1, 4));
        assert_eq!(p.guid[0], 9);
    }

    #[test]
    fn handles_are_sign_extended() {
        let mut buf = packet(NIM_ADD, 0xFFFF_FFF0, 1, 0);
        put(&mut buf, ICON, 0x8000_0001);
        let p = parse_notify(&buf).unwrap();
        assert_eq!(p.hwnd, -16);
        assert_eq!(p.icon_handle, i32::MIN as isize + 1);
    }

    #[test]
    fn only_a_modify_packet_is_a_modify() {
        assert!(parse_notify(&packet(NIM_MODIFY, 5, 1, 0))
            .unwrap()
            .is_modify());
        assert!(!parse_notify(&packet(NIM_ADD, 5, 1, 0)).unwrap().is_modify());
    }

    #[test]
    fn a_short_packet_is_read_with_zeros_for_the_rest() {
        let mut buf = packet(NIM_MODIFY, 5, 1, NIF_TIP);
        put_tip(&mut buf, "short");
        buf.truncate(TIP + 12);
        let p = parse_notify(&buf).unwrap();
        assert_eq!(p.hwnd, 5);
        assert_eq!(p.tip, "short");
        assert_eq!(p.version, 0);
        assert!(parse_notify(&buf[..TIP - 1]).is_none());
    }

    #[test]
    fn an_add_becomes_a_set_with_only_the_flagged_fields() {
        let mut buf = packet(NIM_ADD, 3, 1, NIF_TIP | NIF_MESSAGE);
        put(&mut buf, CALLBACK, 0x500);
        put_tip(&mut buf, "tip");
        let p = parse_notify(&buf).unwrap();
        let changes = p.changes(None);
        let [TrayChange::Set(update)] = changes.as_slice() else {
            panic!("expected one set, got {changes:?}");
        };
        assert_eq!(update.tooltip.as_deref(), Some("tip"));
        assert_eq!(update.callback_message, Some(0x500));
        assert_eq!(update.hidden, None);
        assert!(update.icon.is_none());
    }

    #[test]
    fn hidden_needs_the_state_flag_and_the_mask() {
        let mut buf = packet(NIM_MODIFY, 3, 1, NIF_STATE);
        put(&mut buf, STATE, NIS_HIDDEN);
        let no_mask = parse_notify(&buf).unwrap().changes(None);
        assert!(matches!(&no_mask[0], TrayChange::Set(u) if u.hidden.is_none()));
        put(&mut buf, STATE_MASK, NIS_HIDDEN);
        let hidden = parse_notify(&buf).unwrap().changes(None);
        assert!(matches!(&hidden[0], TrayChange::Set(u) if u.hidden == Some(true)));
        put(&mut buf, STATE, 0);
        let shown = parse_notify(&buf).unwrap().changes(None);
        assert!(matches!(&shown[0], TrayChange::Set(u) if u.hidden == Some(false)));
    }

    #[test]
    fn delete_and_version_messages_map_to_their_changes() {
        let key = TrayKey { owner: 3, id: 1 };
        let del = parse_notify(&packet(NIM_DELETE, 3, 1, 0)).unwrap();
        assert_eq!(del.changes(None), vec![TrayChange::Delete(key)]);

        let mut buf = packet(NIM_SETVERSION, 3, 1, 0);
        put(&mut buf, VERSION, 4);
        let ver = parse_notify(&buf).unwrap();
        assert_eq!(ver.changes(None), vec![TrayChange::SetVersion(key, 4)]);
        put(&mut buf, VERSION, 9);
        assert!(parse_notify(&buf).unwrap().changes(None).is_empty());
    }

    #[test]
    fn an_add_with_a_version_also_sets_it() {
        let mut buf = packet(NIM_ADD, 3, 1, NIF_MESSAGE);
        put(&mut buf, VERSION, 3);
        let changes = parse_notify(&buf).unwrap().changes(None);
        assert_eq!(changes.len(), 2);
        assert_eq!(
            changes[1],
            TrayChange::SetVersion(TrayKey { owner: 3, id: 1 }, 3)
        );
    }

    #[test]
    fn packets_without_an_owner_are_refused() {
        let p = parse_notify(&packet(NIM_ADD, 0, 1, NIF_TIP)).unwrap();
        assert!(!p.is_valid());
        assert!(p.changes(None).is_empty());
    }

    #[test]
    fn only_adds_and_modifies_with_an_icon_handle_want_pixels() {
        let mut buf = packet(NIM_ADD, 3, 1, NIF_ICON);
        assert!(!parse_notify(&buf).unwrap().wants_icon());
        put(&mut buf, ICON, 0x20);
        assert!(parse_notify(&buf).unwrap().wants_icon());
        put(&mut buf, MESSAGE, NIM_DELETE);
        assert!(!parse_notify(&buf).unwrap().wants_icon());
    }
}
