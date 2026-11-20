//! Which keymap actions still run while the folder tree has the keyboard:
//! those about the window or the folder, and the strip's previous / next.
//! Keyed by action name so it holds however the keys are rebound; an action
//! missing here (including one added later) is swallowed. After
//! `treekeys.ts`, but unlike it lets `previous` / `next` through, so Left /
//! Right page the strip while the tree keeps the keyboard.

pub const TREE_PASSTHROUGH: [&str; 7] = [
    "open",
    "previous",
    "next",
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

/// After `treekeys.test.ts`, with `previous` / `next` passing.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lets_only_the_window_folder_and_paging_actions_through() {
        let mut passing = TREE_PASSTHROUGH.to_vec();
        passing.sort_unstable();
        assert_eq!(
            passing,
            [
                "next",
                "open",
                "previous",
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
        for action in [
            "pick",
            "undo",
            "selectAll",
            "rate1",
            "red",
            "burstNext",
            "burstFramePrevious",
        ] {
            assert_eq!(tree_gate(Some(action)), Gate::Swallow);
        }
    }

    #[test]
    fn swallows_an_unknown_or_unbound_action() {
        assert_eq!(tree_gate(Some("somethingNew")), Gate::Swallow);
        assert_eq!(tree_gate(None), Gate::Swallow);
    }
}
