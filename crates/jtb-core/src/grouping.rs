use std::collections::HashSet;

use crate::config::{GroupConfig, GroupingMode, PinConfig};
use crate::tasks::Task;

/// How windows are sorted into groups.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    groups: Vec<GroupConfig>,
}

/// A group's identity: `key` decides membership, `name` is what the header shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupId {
    pub key: String,
    pub name: String,
}

fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

fn file_stem(path: &str) -> &str {
    let name = file_name(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

fn entry_matches(entry: &str, exe: &str) -> bool {
    if entry.contains(['\\', '/']) {
        entry.eq_ignore_ascii_case(exe)
    } else {
        entry.eq_ignore_ascii_case(file_name(exe))
    }
}

impl Rules {
    pub fn new(groups: &[GroupConfig]) -> Rules {
        Rules {
            groups: groups.to_vec(),
        }
    }

    /// Group of a window. Windows without a known exe, and UWP windows (all hosted by one
    /// process), only ever form a group of their own.
    pub fn group_of(&self, task: &Task) -> GroupId {
        let alone = GroupId {
            key: format!("win:{}", task.id),
            name: task.title.clone(),
        };
        if task.exe.is_empty()
            || file_name(&task.exe).eq_ignore_ascii_case("applicationframehost.exe")
        {
            return alone;
        }
        self.group_of_exe(&task.exe)
    }

    /// Group an executable belongs to; also used for pins.
    pub fn group_of_exe(&self, exe: &str) -> GroupId {
        for group in &self.groups {
            if group.exes.iter().any(|entry| entry_matches(entry, exe)) {
                return GroupId {
                    key: format!("group:{}", group.name.to_lowercase()),
                    name: group.name.clone(),
                };
            }
        }
        GroupId {
            key: format!("exe:{}", exe.to_lowercase()),
            name: file_stem(exe).to_string(),
        }
    }
}

/// One button of the bar, in drawing order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Window {
        id: isize,
        /// Shown under a header.
        child: bool,
    },
    Header {
        key: String,
        name: String,
        /// Index into the pins this header stands for.
        pin: Option<usize>,
        windows: Vec<isize>,
        folded: bool,
    },
}

struct Group {
    id: GroupId,
    pin: Option<usize>,
    windows: Vec<isize>,
}

/// Turns the window list into buttons. Pins come first, then groups in the order their first
/// window appeared; `order` (group keys) overrides that. A group of one window without a pin is a
/// plain button.
pub fn build_rows(
    tasks: &[Task],
    pins: &[PinConfig],
    rules: &Rules,
    folded: &HashSet<String>,
    order: &[String],
    mode: GroupingMode,
) -> Vec<Row> {
    if mode == GroupingMode::Off {
        return tasks
            .iter()
            .map(|t| Row::Window {
                id: t.id,
                child: false,
            })
            .collect();
    }

    let mut groups: Vec<Group> = Vec::new();
    for (index, pin) in pins.iter().enumerate() {
        let mut id = rules.group_of_exe(&pin.path);
        if groups.iter().any(|g| g.id.key == id.key) {
            continue;
        }
        if !pin.name.is_empty() {
            id.name = pin.name.clone();
        }
        groups.push(Group {
            id,
            pin: Some(index),
            windows: Vec::new(),
        });
    }
    for task in tasks {
        let id = rules.group_of(task);
        match groups.iter_mut().find(|g| g.id.key == id.key) {
            Some(group) => group.windows.push(task.id),
            None => groups.push(Group {
                id,
                pin: None,
                windows: vec![task.id],
            }),
        }
    }
    groups.sort_by_key(|g| {
        order
            .iter()
            .position(|key| *key == g.id.key)
            .unwrap_or(usize::MAX)
    });

    let mut rows = Vec::new();
    for group in groups {
        if group.pin.is_none() && group.windows.len() == 1 {
            rows.push(Row::Window {
                id: group.windows[0],
                child: false,
            });
            continue;
        }
        let is_folded = folded.contains(&group.id.key);
        rows.push(Row::Header {
            key: group.id.key,
            name: group.id.name,
            pin: group.pin,
            windows: group.windows.clone(),
            folded: is_folded,
        });
        if !is_folded {
            rows.extend(
                group
                    .windows
                    .into_iter()
                    .map(|id| Row::Window { id, child: true }),
            );
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::Tasks;

    fn tasks(list: &[(isize, &str)]) -> Tasks {
        let mut tasks = Tasks::default();
        for (id, exe) in list {
            tasks.upsert(*id, format!("window {id}"));
            tasks.set_exe(*id, (*exe).to_string());
        }
        tasks
    }

    fn rows(tasks: &[Task], pins: &[PinConfig], groups: &[GroupConfig]) -> Vec<Row> {
        build_rows(
            tasks,
            pins,
            &Rules::new(groups),
            &HashSet::new(),
            &[],
            GroupingMode::Accordion,
        )
    }

    fn window(id: isize, child: bool) -> Row {
        Row::Window { id, child }
    }

    fn pin(name: &str, path: &str) -> PinConfig {
        PinConfig {
            name: name.into(),
            path: path.into(),
            args: Vec::new(),
        }
    }

    #[test]
    fn single_windows_stay_plain() {
        let t = tasks(&[(1, r"C:\a\one.exe"), (2, r"C:\a\two.exe")]);
        assert_eq!(
            rows(t.items(), &[], &[]),
            vec![window(1, false), window(2, false)]
        );
    }

    #[test]
    fn windows_of_one_exe_get_a_header() {
        let t = tasks(&[
            (1, r"C:\a\one.exe"),
            (2, r"C:\a\two.exe"),
            (3, r"c:\A\ONE.exe"),
        ]);
        let got = rows(t.items(), &[], &[]);
        assert_eq!(
            got,
            vec![
                Row::Header {
                    key: r"exe:c:\a\one.exe".into(),
                    name: "one".into(),
                    pin: None,
                    windows: vec![1, 3],
                    folded: false,
                },
                window(1, true),
                window(3, true),
                window(2, false),
            ]
        );
    }

    #[test]
    fn folded_group_hides_its_children() {
        let t = tasks(&[(1, r"C:\a\one.exe"), (2, r"C:\a\one.exe")]);
        let folded = HashSet::from([r"exe:c:\a\one.exe".to_string()]);
        let got = build_rows(
            t.items(),
            &[],
            &Rules::default(),
            &folded,
            &[],
            GroupingMode::Accordion,
        );
        assert_eq!(got.len(), 1);
        assert!(matches!(&got[0], Row::Header { folded: true, windows, .. } if windows.len() == 2));
    }

    #[test]
    fn windows_without_exe_and_uwp_hosts_never_group() {
        let t = tasks(&[
            (1, ""),
            (2, ""),
            (3, r"C:\Windows\System32\ApplicationFrameHost.exe"),
            (4, r"C:\Windows\System32\ApplicationFrameHost.exe"),
        ]);
        assert!(rows(t.items(), &[], &[])
            .iter()
            .all(|r| matches!(r, Row::Window { child: false, .. })));
    }

    #[test]
    fn pins_come_first_and_claim_their_windows() {
        let t = tasks(&[(1, r"C:\a\one.exe"), (2, r"C:\a\two.exe")]);
        let pins = [pin("Two", r"C:\a\two.exe"), pin("", r"C:\a\idle.exe")];
        let got = rows(t.items(), &pins, &[]);
        assert_eq!(got.len(), 4);
        assert!(
            matches!(&got[0], Row::Header { name, pin: Some(0), windows, .. } if name == "Two" && windows == &vec![2])
        );
        assert_eq!(got[1], window(2, true));
        assert!(
            matches!(&got[2], Row::Header { name, pin: Some(1), windows, .. } if name == "idle" && windows.is_empty())
        );
        assert_eq!(got[3], window(1, false));
    }

    #[test]
    fn custom_group_merges_different_exes() {
        let t = tasks(&[
            (1, r"C:\p\PuTTYPortable.exe"),
            (2, r"D:\x\putty.exe"),
            (3, r"C:\o.exe"),
        ]);
        let groups = [GroupConfig {
            name: "PuTTY".into(),
            exes: vec!["puttyportable.exe".into(), "putty.exe".into()],
        }];
        let got = rows(t.items(), &[], &groups);
        assert!(matches!(&got[0], Row::Header { key, name, windows, .. }
            if key == "group:putty" && name == "PuTTY" && windows == &vec![1, 2]));
        assert_eq!(got.last(), Some(&window(3, false)));
    }

    #[test]
    fn a_pin_claims_a_custom_group() {
        let t = tasks(&[(1, r"C:\p\PuTTYPortable.exe"), (2, r"D:\x\putty.exe")]);
        let groups = [GroupConfig {
            name: "PuTTY".into(),
            exes: vec!["puttyportable.exe".into(), "putty.exe".into()],
        }];
        let got = rows(t.items(), &[pin("", r"E:\tools\putty.exe")], &groups);
        assert!(
            matches!(&got[0], Row::Header { pin: Some(0), windows, .. } if windows == &vec![1, 2])
        );
    }

    #[test]
    fn explicit_order_overrides_the_default() {
        let t = tasks(&[
            (1, r"C:\a\one.exe"),
            (2, r"C:\a\one.exe"),
            (3, r"C:\a\two.exe"),
            (4, r"C:\a\two.exe"),
        ]);
        let order = [r"exe:c:\a\two.exe".to_string()];
        let got = build_rows(
            t.items(),
            &[],
            &Rules::default(),
            &HashSet::new(),
            &order,
            GroupingMode::Accordion,
        );
        assert!(matches!(&got[0], Row::Header { name, .. } if name == "two"));
    }

    #[test]
    fn off_mode_is_one_button_per_window() {
        let t = tasks(&[(1, r"C:\a\one.exe"), (2, r"C:\a\one.exe")]);
        let got = build_rows(
            t.items(),
            &[pin("", r"C:\a\one.exe")],
            &Rules::default(),
            &HashSet::new(),
            &[],
            GroupingMode::Off,
        );
        assert_eq!(got, vec![window(1, false), window(2, false)]);
    }
}
