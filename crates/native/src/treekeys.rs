//! Which keymap actions still run while the folder tree has the keyboard:
//! only those about the window or the folder, never about a photo. Keyed by
//! action name so it holds however the keys are rebound; an action missing
//! here (including one added later) is swallowed. A port of `treekeys.ts`.

pub const TREE_PASSTHROUGH: [&str; 5] = [
    "open",
    "toggleLeft",
    "toggleRight",
    "toggleStrip",
    "toggleSides",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Run,
    Swallow,
}

pub fn tree_gate(action: Option<&str>) -> Gate {
    match action {
        Some(action) if TREE_PASSTHROUGH.contains(&action) => Gate::Run,
        _ => Gate::Swallow,
    }
}

/// Ported from `treekeys.test.ts`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lets_only_the_window_and_folder_actions_through() {
        let mut passing = TREE_PASSTHROUGH.to_vec();
        passing.sort_unstable();
        assert_eq!(
            passing,
            [
                "open",
                "toggleLeft",
                "toggleRight",
                "toggleSides",
                "toggleStrip"
            ]
        );
        for action in TREE_PASSTHROUGH {
            assert_eq!(tree_gate(Some(action)), Gate::Run);
        }
    }

    #[test]
    fn swallows_the_culling_actions() {
        for action in ["pick", "next", "undo", "selectAll", "rate1", "red"] {
            assert_eq!(tree_gate(Some(action)), Gate::Swallow);
        }
    }

    #[test]
    fn swallows_an_unknown_or_unbound_action() {
        assert_eq!(tree_gate(Some("somethingNew")), Gate::Swallow);
        assert_eq!(tree_gate(None), Gate::Swallow);
    }
}
