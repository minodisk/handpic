use gpui_kit::Modifiers;

/// The momentary grayscale preview: on from the `grayscale` key's press until
/// that key or a modifier held with it goes up, or a forced release. An MCP
/// client's `set_overlays` latches it on instead, until it sets it off or the
/// user's own press of the key goes up. Each method says whether the state
/// changed, so a repeated press redraws nothing.
#[derive(Default)]
pub struct Hold {
    /// The held key's `Keystroke.key`, lower-cased; GPUI has no physical code.
    key: Option<String>,
    modifiers: Modifiers,
    latched: bool,
}

impl Hold {
    pub fn press(&mut self, key: &str, modifiers: Modifiers) -> bool {
        let key = key.to_lowercase();
        self.modifiers = modifiers;
        let changed = self.key.as_deref() != Some(key.as_str());
        self.key = Some(key);
        changed
    }

    /// The held key's release ends a latch too.
    pub fn key_up(&mut self, key: &str) -> bool {
        if self.key.as_deref() == Some(key.to_lowercase().as_str()) {
            self.key = None;
            self.latched = false;
            true
        } else {
            false
        }
    }

    /// A modifier going up ends the hold; one going down is only remembered.
    pub fn modifiers_changed(&mut self, modifiers: Modifiers) -> bool {
        let old = self.modifiers;
        self.modifiers = modifiers;
        let lifted = (old.control && !modifiers.control)
            || (old.alt && !modifiers.alt)
            || (old.shift && !modifiers.shift)
            || (old.platform && !modifiers.platform)
            || (old.function && !modifiers.function);
        lifted && self.release()
    }

    /// Ends the hold of a key whose release may never arrive; a latch stays.
    pub fn release(&mut self) -> bool {
        self.key.take().is_some() && !self.latched
    }

    /// Latch it on, or turn it off, held or latched.
    pub fn set(&mut self, on: bool) -> bool {
        let was = self.is_on();
        self.latched = on;
        if !on {
            self.key = None;
        }
        was != on
    }

    pub fn is_on(&self) -> bool {
        self.key.is_some() || self.latched
    }
}

#[cfg(test)]
mod tests {
    use super::Hold;
    use gpui_kit::Modifiers;

    fn ctrl() -> Modifiers {
        Modifiers {
            control: true,
            ..Modifiers::default()
        }
    }

    #[test]
    fn press_turns_it_on_once() {
        let mut hold = Hold::default();
        assert!(!hold.is_on());
        assert!(hold.press("g", Modifiers::none()));
        assert!(hold.is_on());
        assert!(!hold.press("g", Modifiers::none()));
        assert!(hold.is_on());
    }

    #[test]
    fn key_up_of_the_held_key_ends_it() {
        let mut hold = Hold::default();
        hold.press("g", Modifiers::none());
        assert!(hold.key_up("G"));
        assert!(!hold.is_on());
        assert!(!hold.key_up("g"));
    }

    #[test]
    fn key_up_of_another_key_keeps_it() {
        let mut hold = Hold::default();
        hold.press("g", Modifiers::none());
        assert!(!hold.key_up("h"));
        assert!(hold.is_on());
    }

    #[test]
    fn modifier_going_down_keeps_it() {
        let mut hold = Hold::default();
        hold.press("g", Modifiers::none());
        assert!(!hold.modifiers_changed(ctrl()));
        assert!(hold.is_on());
        assert!(hold.modifiers_changed(Modifiers::none()));
        assert!(!hold.is_on());
    }

    #[test]
    fn modifier_going_up_ends_it() {
        let mut hold = Hold::default();
        hold.press("g", ctrl());
        assert!(hold.modifiers_changed(Modifiers::none()));
        assert!(!hold.is_on());
        assert!(!hold.modifiers_changed(Modifiers::none()));
    }

    #[test]
    fn rebound_key_ends_on_its_key_first() {
        let mut hold = Hold::default();
        hold.press("g", ctrl());
        assert!(hold.key_up("g"));
        assert!(!hold.modifiers_changed(Modifiers::none()));
        assert!(!hold.is_on());
    }

    #[test]
    fn release_forces_it_off() {
        let mut hold = Hold::default();
        assert!(!hold.release());
        hold.press("g", Modifiers::none());
        assert!(hold.release());
        assert!(!hold.is_on());
    }

    #[test]
    fn a_latch_outlasts_a_release_until_set_off() {
        let mut hold = Hold::default();
        assert!(hold.set(true));
        assert!(!hold.set(true));
        assert!(!hold.release());
        assert!(!hold.modifiers_changed(Modifiers::none()));
        assert!(hold.is_on());
        assert!(hold.set(false));
        assert!(!hold.is_on());
        assert!(!hold.set(false));
    }

    #[test]
    fn the_keys_release_ends_a_latch() {
        let mut hold = Hold::default();
        hold.set(true);
        hold.press("g", Modifiers::none());
        assert!(!hold.release());
        assert!(hold.is_on());
        hold.press("g", Modifiers::none());
        assert!(hold.key_up("g"));
        assert!(!hold.is_on());
    }
}
