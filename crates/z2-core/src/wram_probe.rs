//! WRAM access probe (feature `wram-probe`): which bytes of `$6000-$7FFF`
//! the game ever reads or writes.
//!
//! The interpreter bus ([`crate::cpu`] `bus_read`/`bus_write`) reports every
//! `$6000-$7FFF` access here while a probe is armed on the current thread.
//! Ported trap bodies that go through `bus_read`/`bus_write` are counted the
//! same way; the few that index `Game::wram` directly are not, which is why
//! the survey driver (`cargo xtask content-survey`) also replays each movie
//! with the trap table off (pure interpretation of the original code) and
//! diffs the trapped run's WRAM frame by frame. See
//! README.md for the method and results.
//!
//! Off by default: without the feature the bus carries no probe call at all.

use std::cell::RefCell;

/// Bytes in the probed window (`$6000-$7FFF`).
pub const LEN: usize = 0x2000;

/// Per-byte access record for one armed probe.
#[derive(Debug, Clone)]
pub struct WramAccess {
    /// Reads per byte (saturating).
    pub reads: Vec<u32>,
    /// Writes per byte (saturating).
    pub writes: Vec<u32>,
    /// Frame of the first read (`u32::MAX` = never).
    pub first_read: Vec<u32>,
    /// Frame of the first write (`u32::MAX` = never).
    pub first_write: Vec<u32>,
    /// Frame of the last write (`u32::MAX` = never).
    pub last_write: Vec<u32>,
    /// Frame number stamped on accesses (set by the driver each frame).
    pub frame: u32,
}

impl WramAccess {
    fn new() -> Self {
        Self {
            reads: vec![0; LEN],
            writes: vec![0; LEN],
            first_read: vec![u32::MAX; LEN],
            first_write: vec![u32::MAX; LEN],
            last_write: vec![u32::MAX; LEN],
            frame: 0,
        }
    }
}

thread_local! {
    static PROBE: RefCell<Option<WramAccess>> = const { RefCell::new(None) };
}

/// Arm a fresh probe on this thread (drops any previous record).
pub fn arm() {
    PROBE.with(|p| *p.borrow_mut() = Some(WramAccess::new()));
}

/// Disarm and return the record (`None` if no probe was armed).
pub fn take() -> Option<WramAccess> {
    PROBE.with(|p| p.borrow_mut().take())
}

/// Re-arm with a record taken earlier (lets one thread interleave two
/// games, each with its own record).
pub fn restore(record: WramAccess) {
    PROBE.with(|p| *p.borrow_mut() = Some(record));
}

/// Stamp subsequent accesses with `frame`.
pub fn set_frame(frame: u32) {
    PROBE.with(|p| {
        if let Some(a) = p.borrow_mut().as_mut() {
            a.frame = frame;
        }
    });
}

/// Bus hook: one read of `$6000 + off`.
#[inline]
pub(crate) fn note_read(off: usize) {
    PROBE.with(|p| {
        if let Some(a) = p.borrow_mut().as_mut() {
            a.reads[off] = a.reads[off].saturating_add(1);
            if a.first_read[off] == u32::MAX {
                a.first_read[off] = a.frame;
            }
        }
    });
}

/// Bus hook: one write of `$6000 + off`.
#[inline]
pub(crate) fn note_write(off: usize) {
    PROBE.with(|p| {
        if let Some(a) = p.borrow_mut().as_mut() {
            a.writes[off] = a.writes[off].saturating_add(1);
            if a.first_write[off] == u32::MAX {
                a.first_write[off] = a.frame;
            }
            a.last_write[off] = a.frame;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_records_only_while_armed() {
        note_read(5);
        arm();
        set_frame(3);
        note_read(5);
        note_write(7);
        set_frame(9);
        note_write(7);
        let a = take().expect("armed");
        assert_eq!(a.reads[5], 1);
        assert_eq!(a.first_read[5], 3);
        assert_eq!(a.writes[7], 2);
        assert_eq!((a.first_write[7], a.last_write[7]), (3, 9));
        assert!(take().is_none());
        note_write(1); // disarmed: no panic, nothing recorded
    }
}
