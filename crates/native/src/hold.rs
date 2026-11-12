use std::{cell::RefCell, rc::Rc};

use riffle_core::scan::Paused;

/// The scan's pause a preview holds from its read until its image is
/// painted, so pass 2's workers leave the cores to the decode, the orient and
/// the paint as well as the disk to the read (`ReadSlots::wait_unpaused` at
/// their stage boundaries). `MAX_PAUSE` still bounds what the workers honour
/// (about 1 s for a pass 2 file's stages in all, and 1 s per `acquire`, 3 s
/// per file at worst), so a hold that is never released, for instance when no
/// frame is painted, only slows the scan.
#[derive(Clone, Default)]
pub struct PaintHold(Rc<RefCell<Option<Paused>>>);

impl PaintHold {
    /// Hold `paused` until `release`; a pause already held ends now.
    pub fn hold(&self, paused: Paused) {
        *self.0.borrow_mut() = Some(paused);
    }

    /// The held image was painted: end the pause.
    pub fn release(&self) {
        self.0.borrow_mut().take();
    }
}

#[cfg(test)]
mod tests {
    use super::PaintHold;
    use riffle_core::scan::ReadSlots;
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        thread,
        time::{Duration, Instant},
    };

    /// How long `wait_unpaused` waited, a helper thread cancelling the wait
    /// after `cancel_after` so a held pause does not take the whole maximum.
    fn waited(slots: &ReadSlots, cancel_after: Duration) -> Duration {
        let cancel = AtomicBool::new(false);
        let started = Instant::now();
        thread::scope(|s| {
            s.spawn(|| {
                thread::sleep(cancel_after);
                cancel.store(true, Ordering::Relaxed);
            });
            slots.wait_unpaused(&cancel);
        });
        started.elapsed()
    }

    #[test]
    fn a_held_pause_keeps_the_workers_waiting_until_released() {
        let slots = ReadSlots::new(1);
        let hold = PaintHold::default();
        hold.hold(slots.pause());
        assert!(waited(&slots, Duration::from_millis(100)) >= Duration::from_millis(100));
        hold.release();
        let started = Instant::now();
        slots.wait_unpaused(&AtomicBool::new(false));
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn a_new_hold_ends_the_previous_one() {
        let slots = ReadSlots::new(1);
        let hold = PaintHold::default();
        hold.hold(slots.pause());
        hold.hold(slots.pause());
        hold.release();
        let started = Instant::now();
        slots.wait_unpaused(&AtomicBool::new(false));
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}
