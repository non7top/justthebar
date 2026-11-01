/// One window shown on the bar. `id` is the window handle value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: isize,
    pub title: String,
}

/// The windows on the taskbar in creation order, and which one is active.
#[derive(Debug, Default)]
pub struct Tasks {
    items: Vec<Task>,
    active: Option<isize>,
    flashing: Vec<isize>,
}

impl Tasks {
    pub fn items(&self) -> &[Task] {
        &self.items
    }

    pub fn active(&self) -> Option<isize> {
        self.active
    }

    /// Adds the window, or updates its title if it is already listed.
    pub fn upsert(&mut self, id: isize, title: String) {
        match self.items.iter_mut().find(|t| t.id == id) {
            Some(task) => task.title = title,
            None => self.items.push(Task { id, title }),
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
        self.items.retain(|t| t.id != id);
        if self.active == Some(id) {
            self.active = None;
        }
    }

    pub fn contains(&self, id: isize) -> bool {
        self.items.iter().any(|t| t.id == id)
    }

    pub fn set_active(&mut self, id: Option<isize>) {
        self.active = id;
        if let Some(id) = id {
            self.flashing.retain(|f| *f != id);
        }
    }

    /// Replaces the list, keeping the order of windows that were already listed.
    pub fn replace_all(&mut self, tasks: Vec<Task>) {
        self.items.retain(|t| tasks.iter().any(|n| n.id == t.id));
        self.flashing.retain(|f| tasks.iter().any(|n| n.id == *f));
        for task in tasks {
            self.upsert(task.id, task.title);
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
