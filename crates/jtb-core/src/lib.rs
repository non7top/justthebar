pub mod activity;
pub mod buttons;
pub mod clock;
pub mod config;
pub mod grouping;
pub mod icon;
pub mod layout;
pub mod scene;
pub mod settings;
pub mod startart;
pub mod state;
pub mod tasks;
pub mod tray;
pub mod traypacket;
pub mod widget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Edge {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Edge::Top | Edge::Bottom)
    }

    pub fn parse(s: &str) -> Option<Edge> {
        match s {
            "top" => Some(Edge::Top),
            "bottom" => Some(Edge::Bottom),
            "left" => Some(Edge::Left),
            "right" => Some(Edge::Right),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_edges() {
        assert!(Edge::Top.is_horizontal());
        assert!(Edge::Bottom.is_horizontal());
        assert!(!Edge::Left.is_horizontal());
        assert!(!Edge::Right.is_horizontal());
    }

    #[test]
    fn parse_edges() {
        assert_eq!(Edge::parse("top"), Some(Edge::Top));
        assert_eq!(Edge::parse("right"), Some(Edge::Right));
        assert_eq!(Edge::parse("middle"), None);
    }
}
