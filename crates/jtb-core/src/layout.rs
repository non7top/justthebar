use crate::Edge;

/// One bar's place on a monitor, used to resolve how bars share the corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub monitor: usize,
    pub edge: Edge,
    pub thickness: i32,
}

/// How far a bar is shortened at each end of its long side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Insets {
    /// Left end of a horizontal bar, top end of a vertical one.
    pub start: i32,
    /// Right end of a horizontal bar, bottom end of a vertical one.
    pub end: i32,
}

/// Earlier slots win the corners: a later bar on a perpendicular edge of the same monitor is
/// shortened by the thickness of the earlier ones.
pub fn corner_insets(slots: &[Slot]) -> Vec<Insets> {
    slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let mut insets = Insets::default();
            for earlier in &slots[..i] {
                if earlier.monitor != slot.monitor
                    || earlier.edge.is_horizontal() == slot.edge.is_horizontal()
                {
                    continue;
                }
                match earlier.edge {
                    Edge::Left | Edge::Top => insets.start += earlier.thickness,
                    Edge::Right | Edge::Bottom => insets.end += earlier.thickness,
                }
            }
            insets
        })
        .collect()
}

/// Distance of each bar from its screen edge: the thickness of earlier bars on the same edge.
pub fn edge_offsets(slots: &[Slot]) -> Vec<i32> {
    slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            slots[..i]
                .iter()
                .filter(|e| e.monitor == slot.monitor && e.edge == slot.edge)
                .map(|e| e.thickness)
                .sum()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(monitor: usize, edge: Edge, thickness: i32) -> Slot {
        Slot {
            monitor,
            edge,
            thickness,
        }
    }

    #[test]
    fn first_bar_is_full_length() {
        let insets = corner_insets(&[slot(0, Edge::Bottom, 40)]);
        assert_eq!(insets, vec![Insets::default()]);
    }

    #[test]
    fn later_perpendicular_bar_is_shortened() {
        let insets = corner_insets(&[slot(0, Edge::Bottom, 40), slot(0, Edge::Left, 30)]);
        assert_eq!(insets[0], Insets::default());
        assert_eq!(insets[1], Insets { start: 0, end: 40 });
    }

    #[test]
    fn other_monitors_and_parallel_edges_do_not_interact() {
        let insets = corner_insets(&[
            slot(0, Edge::Bottom, 40),
            slot(1, Edge::Left, 30),
            slot(0, Edge::Bottom, 20),
        ]);
        assert!(insets.iter().all(|i| *i == Insets::default()));
    }

    #[test]
    fn three_bars_accumulate() {
        let insets = corner_insets(&[
            slot(0, Edge::Left, 30),
            slot(0, Edge::Right, 20),
            slot(0, Edge::Top, 10),
        ]);
        assert_eq!(insets[2], Insets { start: 30, end: 20 });
    }

    #[test]
    fn same_edge_bars_stack() {
        let slots = [
            slot(0, Edge::Right, 40),
            slot(0, Edge::Right, 20),
            slot(0, Edge::Bottom, 10),
            slot(1, Edge::Right, 5),
        ];
        assert_eq!(edge_offsets(&slots), vec![0, 40, 0, 0]);
    }
}
