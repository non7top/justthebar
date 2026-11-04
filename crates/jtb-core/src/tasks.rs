use crate::activity::ActivityState;

/// One window shown on the bar. `id` is the window handle value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: isize,
    pub title: String,
    /// Full path of the window's executable; empty when it could not be read.
    pub exe: String,
    pub activity: ActivityState,
}

/// The windows on the taskbar in creation order, and which one is active.
#[derive(Debug, Default)]
pub struct Tasks {
    items: Vec<Task>,
    active: Option<isize>,
    flashing: Vec<isize>,
    title_changed: Vec<isize>,
    status_changed: Vec<isize>,
}

impl Tasks {
    pub fn items(&self) -> &[Task] {
        &self.items
    }

    pub fn active(&self) -> Option<isize> {
        self.active
    }

    /// Adds the window, or updates its title if it is already listed. A new title on a window
    /// that is not active is remembered until the window is activated.
    pub fn upsert(&mut self, id: isize, title: String) {
        match self.items.iter_mut().find(|t| t.id == id) {
            Some(task) => {
                if task.title != title
                    && self.active != Some(id)
                    && !self.title_changed.contains(&id)
                    && task.activity != ActivityState::Working
                {
                    self.title_changed.push(id);
                }
                task.title = title;
            }
            None => self.items.push(Task {
                id,
                title,
                exe: String::new(),
                activity: ActivityState::default(),
            }),
        }
    }

    pub fn has_new_title(&self, id: isize) -> bool {
        self.title_changed.contains(&id)
    }

    pub fn has_status_changed(&self, id: isize) -> bool {
        self.status_changed.contains(&id)
    }

    pub fn set_status_changed(&mut self, id: isize, changed: bool) {
        self.status_changed.retain(|f| *f != id);
        if changed && self.active != Some(id) {
            self.status_changed.push(id);
        }
    }

    pub fn activity_state(&self, id: isize) -> ActivityState {
        self.items
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.activity)
            .unwrap_or_default()
    }

    pub fn set_activity_state(&mut self, id: isize, state: ActivityState) {
        if let Some(task) = self.items.iter_mut().find(|t| t.id == id) {
            task.activity = state;
        }
    }

    pub fn is_flashing(&self, id: isize) -> bool {
        self.flashing.contains(&id)
    }

    /// Marks the window as asking for attention. The active window never flashes.
    pub fn set_flashing(&mut self, id: isize, flashing: bool) {
        self.flashing.retain(|f| *f != id);
        if flashing && self.active != Some(id) {
            self.flashing.push(id);
        }
    }

    pub fn remove(&mut self, id: isize) {
        self.flashing.retain(|f| *f != id);
        self.title_changed.retain(|f| *f != id);
        self.status_changed.retain(|f| *f != id);
        self.items.retain(|t| t.id != id);
        if self.active == Some(id) {
            self.active = None;
        }
    }

    pub fn get(&self, id: isize) -> Option<&Task> {
        self.items.iter().find(|t| t.id == id)
    }

    pub fn set_exe(&mut self, id: isize, exe: String) {
        if let Some(task) = self.items.iter_mut().find(|t| t.id == id) {
            task.exe = exe;
        }
    }

    /// Moves task `from` to where task `to` is. The order lasts until the window closes.
    pub fn move_to(&mut self, from: isize, to: isize) {
        let (Some(from_at), Some(to_at)) = (
            self.items.iter().position(|t| t.id == from),
            self.items.iter().position(|t| t.id == to),
        ) else {
            return;
        };
        let task = self.items.remove(from_at);
        self.items.insert(to_at, task);
    }

    pub fn contains(&self, id: isize) -> bool {
        self.items.iter().any(|t| t.id == id)
    }

    pub fn set_active(&mut self, id: Option<isize>) {
        self.active = id;
        if let Some(id) = id {
            self.flashing.retain(|f| *f != id);
            self.title_changed.retain(|f| *f != id);
            self.status_changed.retain(|f| *f != id);
        }
    }

    /// Replaces the list, keeping the order of windows that were already listed.
    pub fn replace_all(&mut self, tasks: Vec<Task>) {
        self.items.retain(|t| tasks.iter().any(|n| n.id == t.id));
        self.flashing.retain(|f| tasks.iter().any(|n| n.id == *f));
        self.title_changed
            .retain(|f| tasks.iter().any(|n| n.id == *f));
        self.status_changed
            .retain(|f| tasks.iter().any(|n| n.id == *f));
        for task in tasks {
            self.upsert(task.id, task.title);
            self.set_exe(task.id, task.exe);
            if task.activity != ActivityState::default() {
                self.set_activity_state(task.id, task.activity);
            }
        }
        if self.active.is_some_and(|id| !self.contains(id)) {
            self.active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: isize, title: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            exe: String::new(),
            activity: ActivityState::default(),
        }
    }

    #[test]
    fn upsert_keeps_creation_order_and_updates_titles() {
        let mut tasks = Tasks::default();
        tasks.upsert(1, "one".into());
        tasks.upsert(2, "two".into());
        tasks.upsert(1, "uno".into());
        assert_eq!(tasks.items(), &[task(1, "uno"), task(2, "two")]);
    }

    #[test]
    fn removing_the_active_window_clears_it() {
        let mut tasks = Tasks::default();
        tasks.upsert(1, "one".into());
        tasks.set_active(Some(1));
        tasks.remove(1);
        assert!(tasks.items().is_empty());
        assert_eq!(tasks.active(), None);
    }

    #[test]
    fn activating_a_flashing_window_stops_the_flash() {
        let mut tasks = Tasks::default();
        tasks.upsert(1, "one".into());
        tasks.set_flashing(1, true);
        assert!(tasks.is_flashing(1));
        tasks.set_active(Some(1));
        assert!(!tasks.is_flashing(1));
        tasks.set_flashing(1, true);
        assert!(!tasks.is_flashing(1));
    }

    #[test]
    fn a_new_title_on_an_inactive_window_is_flagged_until_activated() {
        let mut tasks = Tasks::default();
        tasks.upsert(1, "one".into());
        tasks.upsert(2, "two".into());
        tasks.set_active(Some(2));
        tasks.upsert(1, "one".into());
        assert!(!tasks.has_new_title(1));
        tasks.upsert(1, "one (3)".into());
        tasks.upsert(2, "two!".into());
        assert!(tasks.has_new_title(1));
        assert!(!tasks.has_new_title(2));
        tasks.set_active(Some(1));
        assert!(!tasks.has_new_title(1));
    }

    #[test]
    fn status_change_is_cleared_on_activate() {
        let mut tasks = Tasks::default();
        tasks.upsert(1, "one".into());
        tasks.set_status_changed(1, true);
        assert!(tasks.has_status_changed(1));
        tasks.set_active(Some(1));
        assert!(!tasks.has_status_changed(1));
    }

    #[test]
    fn working_task_title_update_does_not_set_title_changed() {
        let mut tasks = Tasks::default();
        tasks.upsert(1, "step 1".into());
        tasks.set_activity_state(1, ActivityState::Working);
        tasks.upsert(1, "step 2".into());
        assert!(!tasks.has_new_title(1));
    }

    #[test]
    fn move_to_puts_a_task_where_the_other_is() {
        let mut tasks = Tasks::default();
        for id in 1..=3 {
            tasks.upsert(id, format!("w{id}"));
        }
        tasks.move_to(1, 3);
        let ids: Vec<isize> = tasks.items().iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![2, 3, 1]);
        tasks.move_to(1, 99);
        assert_eq!(tasks.items().len(), 3);
    }

    #[test]
    fn replace_all_drops_missing_and_keeps_order() {
        let mut tasks = Tasks::default();
        tasks.upsert(1, "one".into());
        tasks.upsert(2, "two".into());
        tasks.set_active(Some(2));
        tasks.replace_all(vec![task(3, "three"), task(1, "one!")]);
        assert_eq!(tasks.items(), &[task(1, "one!"), task(3, "three")]);
        assert_eq!(tasks.active(), None);
    }
}
