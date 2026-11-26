//! **The boot keyboard report, and what changed between two of them** (HID 1.11 Appendix B.1,
//! HID Usage Tables 1.5 section 10).
//!
//! A boot keyboard does not send key events. It sends its **state**: eight bytes, the modifier
//! keys as a bitmap in byte 0, a reserved byte, then up to six usage ids for the other keys held
//! down, in no promised order. So a keystroke is a *difference* between two reports, and
//! [`changes`] computes it, as HID usages (the modifiers as 0xe0 to 0xe7). A caller that wants the
//! evdev numbering, so that a keymap which already turns evdev codes (a virtio keyboard's, say)
//! into terminal bytes does the rest, maps each usage through [`evdev_code`].
//!
//! # Where the table comes from
//!
//! [`evdev_code`]'s table, modifier rows included, comes from two public documents, and every row
//! carries both in its comment:
//!
//! - The usage id and its name come from the USB-IF *HID Usage Tables for USB*, version 1.5
//!   (`hut1_5.pdf`, dated 2024-01-08), section 10, the Keyboard/Keypad page. A row's comment
//!   gives the usage name without the page's `Keyboard` prefix.
//! - Each number is the value of the `KEY_*` name that matches that usage name, read from
//!   `include/uapi/linux/input-event-codes.h` at
//!   `torvalds/linux@cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a` (GPL-2.0-only WITH
//!   Linux-syscall-note). That header is an ABI: its numbers are the interface itself, and the
//!   crate takes the numbers and the names, no text.
//!
//! [`evdev_code`] states the matches the Usage Tables settle and the six that are this crate's
//! choices.
//!
//! # Cross-check
//!
//! A test, `the_table_matches_its_linux_cross_check`, records a comparison with Linux's
//! `hid_keyboard[]` and fails when the table changes, so a changed row reruns it. The test's
//! comment gives the revision and the result. The crate copies no Linux driver text.

/// How many bytes a boot report holds.
pub const REPORT_BYTES: usize = 8;

/// The usage id a keyboard puts in every key slot when more keys are held than it can report
/// (`ErrorRollOver`, Usage Tables section 10). A report carrying it describes no real state.
#[doc(alias = "ErrorRollOver")]
pub const ERROR_ROLL_OVER: u8 = 0x01;

/// The usage id of the first modifier key, left control; the eight modifiers are usages 0xe0 to
/// 0xe7, in the bit order of byte 0 of a report.
const FIRST_MODIFIER_USAGE: u8 = 0xe0;

/// The evdev codes of the eight modifier keys, usages 0xe0 to 0xe7, in the bit order of byte 0:
/// left control, shift, alt, graphical user interface (GUI) key, then the right-hand four. The module documentation says where
/// the numbers come from, and [`evdev_code`] why the GUI keys are Meta.
const MODIFIER_CODES: [u16; 8] = [
    29,  // 0xe0 LeftControl: KEY_LEFTCTRL
    42,  // 0xe1 LeftShift: KEY_LEFTSHIFT
    56,  // 0xe2 LeftAlt: KEY_LEFTALT
    125, // 0xe3 Left GUI: KEY_LEFTMETA
    97,  // 0xe4 RightControl: KEY_RIGHTCTRL
    54,  // 0xe5 RightShift: KEY_RIGHTSHIFT
    100, // 0xe6 RightAlt: KEY_RIGHTALT
    126, // 0xe7 Right GUI: KEY_RIGHTMETA
];

/// **One boot report**: the modifier bitmap and the six key slots. Its fields are private, so every
/// value is one [`BootReport::parse`] would accept: no key slot holds an error usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BootReport {
    /// Byte 0: one bit per modifier key, in the order of usages 0xe0 to 0xe7.
    modifiers: u8,
    /// Bytes 2..8: usage ids of the keys held, zero for an empty slot.
    keys: [u8; 6],
}

impl BootReport {
    /// Nothing held: the state before the first report.
    pub const RELEASED: BootReport = BootReport {
        modifiers: 0,
        keys: [0; 6],
    };

    /// Parse a report. `None` for fewer than [`REPORT_BYTES`] bytes, and for a **phantom** report
    /// (any slot holding [`ERROR_ROLL_OVER`] or the two other error usages, 2 and 3), which a
    /// driver must ignore rather than read as every key released. A buffer longer than eight bytes
    /// is accepted and its tail ignored.
    ///
    /// # Errors
    ///
    /// `None` when `bytes` holds fewer than [`REPORT_BYTES`] bytes, and for a phantom report.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::boot_keyboard::{BootReport, ERROR_ROLL_OVER};
    ///
    /// # fn main() {
    /// #     assert!(example().is_some());
    /// # }
    /// # fn example() -> Option<()> {
    /// let report = BootReport::parse(&[0x02, 0, 0x0b, 0, 0, 0, 0, 0])?; // left shift, usage 0x0b (h)
    /// assert_eq!((report.modifiers(), report.keys()), (0x02, [0x0b, 0, 0, 0, 0, 0]));
    /// assert_eq!(BootReport::parse(&[0, 0, ERROR_ROLL_OVER, 0, 0, 0, 0, 0]), None);
    /// # Some(())
    /// # }
    /// ```
    pub fn parse(bytes: &[u8]) -> Option<BootReport> {
        if bytes.len() < REPORT_BYTES {
            return None;
        }
        let mut keys = [0u8; 6];
        keys.copy_from_slice(&bytes[2..8]);
        BootReport::new(bytes[0], keys)
    }

    /// Build a report from its modifier bitmap and its six key slots, as a test of a caller's
    /// driver feeds [`changes`]. `None` for what [`BootReport::parse`] refuses: a slot holding
    /// [`ERROR_ROLL_OVER`] or the two other error usages, 2 and 3.
    ///
    /// # Errors
    ///
    /// `None` when a slot of `keys` holds 1, 2 or 3, the error usages.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::boot_keyboard::BootReport;
    ///
    /// # fn main() {
    /// #     assert!(example().is_some());
    /// # }
    /// # fn example() -> Option<()> {
    /// let report = BootReport::new(0, [0x04, 0, 0, 0, 0, 0])?; // usage 0x04 (a)
    /// assert_eq!(BootReport::parse(&[0, 0, 0x04, 0, 0, 0, 0, 0]), Some(report));
    /// # Some(())
    /// # }
    /// ```
    pub fn new(modifiers: u8, keys: [u8; 6]) -> Option<BootReport> {
        if keys.iter().any(|&k| (1..=3).contains(&k)) {
            return None;
        }
        Some(BootReport { modifiers, keys })
    }

    /// Byte 0: one bit per modifier key, in the order of usages 0xe0 to 0xe7.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::boot_keyboard::BootReport;
    ///
    /// assert_eq!(BootReport::new(0x22, [0; 6]).map(|r| r.modifiers()), Some(0x22)); // both shifts
    /// ```
    pub fn modifiers(&self) -> u8 {
        self.modifiers
    }

    /// Bytes 2 to 7: the usage ids of the keys held, zero for an empty slot, in the order the
    /// device sent them, which promises nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::boot_keyboard::BootReport;
    ///
    /// assert_eq!(BootReport::RELEASED.keys(), [0; 6]);
    /// ```
    pub fn keys(&self) -> [u8; 6] {
        self.keys
    }

    fn holds(&self, usage: u8) -> bool {
        usage != 0 && self.keys.contains(&usage)
    }
}

/// **Usage id to evdev code**, for the keyboard page's usages 0x04 through 0x8b: the main block,
/// the function keys to F24, the navigation cluster, the keypad, the European and Japanese keys,
/// and the editing and volume keys; and for the eight modifiers, 0xe0 to 0xe7. `None` for anything
/// else, including 0 (no key), and for the four usages in the first range with no key (below).
/// The module documentation names the sources.
///
#[doc = include_str!("evdev_code.md")]
///
/// # Examples
///
/// ```
/// use universal_serial_bus::boot_keyboard::evdev_code;
///
/// assert_eq!(evdev_code(0x04), Some(30)); // a is KEY_A
/// assert_eq!(evdev_code(0x74), None); // Execute
/// ```
pub fn evdev_code(usage: u8) -> Option<u16> {
    if let Some(bit) = usage.checked_sub(FIRST_MODIFIER_USAGE) {
        return MODIFIER_CODES.get(usize::from(bit)).copied();
    }
    CODE_BY_USAGE
        .get(usize::from(usage).checked_sub(4)?)
        .copied()
        .flatten()
}

/// Indexed by usage id from 0x04. `None` is a usage in the range with no code.
const CODE_BY_USAGE: [Option<u16>; 0x88] = [
    Some(30),  // 0x04 a and A: KEY_A
    Some(48),  // 0x05 b and B: KEY_B
    Some(46),  // 0x06 c and C: KEY_C
    Some(32),  // 0x07 d and D: KEY_D
    Some(18),  // 0x08 e and E: KEY_E
    Some(33),  // 0x09 f and F: KEY_F
    Some(34),  // 0x0a g and G: KEY_G
    Some(35),  // 0x0b h and H: KEY_H
    Some(23),  // 0x0c i and I: KEY_I
    Some(36),  // 0x0d j and J: KEY_J
    Some(37),  // 0x0e k and K: KEY_K
    Some(38),  // 0x0f l and L: KEY_L
    Some(50),  // 0x10 m and M: KEY_M
    Some(49),  // 0x11 n and N: KEY_N
    Some(24),  // 0x12 o and O: KEY_O
    Some(25),  // 0x13 p and P: KEY_P
    Some(16),  // 0x14 q and Q: KEY_Q
    Some(19),  // 0x15 r and R: KEY_R
    Some(31),  // 0x16 s and S: KEY_S
    Some(20),  // 0x17 t and T: KEY_T
    Some(22),  // 0x18 u and U: KEY_U
    Some(47),  // 0x19 v and V: KEY_V
    Some(17),  // 0x1a w and W: KEY_W
    Some(45),  // 0x1b x and X: KEY_X
    Some(21),  // 0x1c y and Y: KEY_Y
    Some(44),  // 0x1d z and Z: KEY_Z
    Some(2),   // 0x1e 1 and !: KEY_1
    Some(3),   // 0x1f 2 and @: KEY_2
    Some(4),   // 0x20 3 and #: KEY_3
    Some(5),   // 0x21 4 and $: KEY_4
    Some(6),   // 0x22 5 and %: KEY_5
    Some(7),   // 0x23 6 and ^: KEY_6
    Some(8),   // 0x24 7 and &: KEY_7
    Some(9),   // 0x25 8 and *: KEY_8
    Some(10),  // 0x26 9 and (: KEY_9
    Some(11),  // 0x27 0 and ): KEY_0
    Some(28),  // 0x28 Return (ENTER): KEY_ENTER
    Some(1),   // 0x29 ESCAPE: KEY_ESC
    Some(14),  // 0x2a DELETE (Backspace): KEY_BACKSPACE
    Some(15),  // 0x2b Tab: KEY_TAB
    Some(57),  // 0x2c Spacebar: KEY_SPACE
    Some(12),  // 0x2d - and _: KEY_MINUS
    Some(13),  // 0x2e = and +: KEY_EQUAL
    Some(26),  // 0x2f [ and {: KEY_LEFTBRACE
    Some(27),  // 0x30 ] and }: KEY_RIGHTBRACE
    Some(43),  // 0x31 \ and |: KEY_BACKSLASH
    Some(43),  // 0x32 Non-US # and ~: KEY_BACKSLASH, this crate's choice
    Some(39),  // 0x33 ; and :: KEY_SEMICOLON
    Some(40),  // 0x34 ' and ": KEY_APOSTROPHE
    Some(41),  // 0x35 Grave Accent and Tilde: KEY_GRAVE
    Some(51),  // 0x36 , and <: KEY_COMMA
    Some(52),  // 0x37 . and >: KEY_DOT
    Some(53),  // 0x38 / and ?: KEY_SLASH
    Some(58),  // 0x39 Caps Lock: KEY_CAPSLOCK
    Some(59),  // 0x3a F1: KEY_F1
    Some(60),  // 0x3b F2: KEY_F2
    Some(61),  // 0x3c F3: KEY_F3
    Some(62),  // 0x3d F4: KEY_F4
    Some(63),  // 0x3e F5: KEY_F5
    Some(64),  // 0x3f F6: KEY_F6
    Some(65),  // 0x40 F7: KEY_F7
    Some(66),  // 0x41 F8: KEY_F8
    Some(67),  // 0x42 F9: KEY_F9
    Some(68),  // 0x43 F10: KEY_F10
    Some(87),  // 0x44 F11: KEY_F11
    Some(88),  // 0x45 F12: KEY_F12
    Some(99),  // 0x46 PrintScreen: KEY_SYSRQ, this crate's choice
    Some(70),  // 0x47 Scroll Lock: KEY_SCROLLLOCK
    Some(119), // 0x48 Pause: KEY_PAUSE
    Some(110), // 0x49 Insert: KEY_INSERT
    Some(102), // 0x4a Home: KEY_HOME
    Some(104), // 0x4b PageUp: KEY_PAGEUP
    Some(111), // 0x4c Delete Forward: KEY_DELETE
    Some(107), // 0x4d End: KEY_END
    Some(109), // 0x4e PageDown: KEY_PAGEDOWN
    Some(106), // 0x4f RightArrow: KEY_RIGHT
    Some(105), // 0x50 LeftArrow: KEY_LEFT
    Some(108), // 0x51 DownArrow: KEY_DOWN
    Some(103), // 0x52 UpArrow: KEY_UP
    Some(69),  // 0x53 Keypad Num Lock and Clear: KEY_NUMLOCK, this crate's choice
    Some(98),  // 0x54 Keypad /: KEY_KPSLASH
    Some(55),  // 0x55 Keypad *: KEY_KPASTERISK
    Some(74),  // 0x56 Keypad -: KEY_KPMINUS
    Some(78),  // 0x57 Keypad +: KEY_KPPLUS
    Some(96),  // 0x58 Keypad ENTER: KEY_KPENTER
    Some(79),  // 0x59 Keypad 1 and End: KEY_KP1
    Some(80),  // 0x5a Keypad 2 and Down Arrow: KEY_KP2
    Some(81),  // 0x5b Keypad 3 and PageDn: KEY_KP3
    Some(75),  // 0x5c Keypad 4 and Left Arrow: KEY_KP4
    Some(76),  // 0x5d Keypad 5: KEY_KP5
    Some(77),  // 0x5e Keypad 6 and Right Arrow: KEY_KP6
    Some(71),  // 0x5f Keypad 7 and Home: KEY_KP7
    Some(72),  // 0x60 Keypad 8 and Up Arrow: KEY_KP8
    Some(73),  // 0x61 Keypad 9 and PageUp: KEY_KP9
    Some(82),  // 0x62 Keypad 0 and Insert: KEY_KP0
    Some(83),  // 0x63 Keypad . and Delete: KEY_KPDOT
    Some(86),  // 0x64 Non-US \ and |: KEY_102ND
    Some(127), // 0x65 Application: KEY_COMPOSE
    Some(116), // 0x66 Power: KEY_POWER
    Some(117), // 0x67 Keypad =: KEY_KPEQUAL
    Some(183), // 0x68 F13: KEY_F13
    Some(184), // 0x69 F14: KEY_F14
    Some(185), // 0x6a F15: KEY_F15
    Some(186), // 0x6b F16: KEY_F16
    Some(187), // 0x6c F17: KEY_F17
    Some(188), // 0x6d F18: KEY_F18
    Some(189), // 0x6e F19: KEY_F19
    Some(190), // 0x6f F20: KEY_F20
    Some(191), // 0x70 F21: KEY_F21
    Some(192), // 0x71 F22: KEY_F22
    Some(193), // 0x72 F23: KEY_F23
    Some(194), // 0x73 F24: KEY_F24
    None,      // 0x74 Execute: no key, the header has no execute key
    Some(138), // 0x75 Help: KEY_HELP
    Some(139), // 0x76 Menu: KEY_MENU
    Some(353), // 0x77 Select: KEY_SELECT
    Some(128), // 0x78 Stop: KEY_STOP
    Some(129), // 0x79 Again: KEY_AGAIN
    Some(131), // 0x7a Undo: KEY_UNDO
    Some(137), // 0x7b Cut: KEY_CUT
    Some(133), // 0x7c Copy: KEY_COPY
    Some(135), // 0x7d Paste: KEY_PASTE
    Some(136), // 0x7e Find: KEY_FIND
    Some(113), // 0x7f Mute: KEY_MUTE
    Some(115), // 0x80 Volume Up: KEY_VOLUMEUP
    Some(114), // 0x81 Volume Down: KEY_VOLUMEDOWN
    None,      // 0x82 Locking Caps Lock: no key, a locking key
    None,      // 0x83 Locking Num Lock: no key, a locking key
    None,      // 0x84 Locking Scroll Lock: no key, a locking key
    Some(121), // 0x85 Keypad Comma: KEY_KPCOMMA
    Some(117), // 0x86 Keypad Equal Sign: KEY_KPEQUAL, the same key as 0x67
    Some(89),  // 0x87 International1: KEY_RO
    Some(93),  // 0x88 International2: KEY_KATAKANAHIRAGANA
    Some(124), // 0x89 International3: KEY_YEN
    Some(92),  // 0x8a International4: KEY_HENKAN
    Some(94),  // 0x8b International5: KEY_MUHENKAN
];

/// **Whether a key went down or came up.** A later release may name another action, so a `match`
/// on it ends in a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyAction {
    /// The key is held in the later report and was not in the earlier one.
    Press,
    /// The key was held in the earlier report and is not in the later one.
    Release,
}

/// **One key that changed between two reports**: its usage on the Keyboard/Keypad page, and
/// whether it went down or came up. Only [`changes`] builds one, and a later release may add a
/// field, so a pattern on it ends in `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct KeyChange {
    /// The key's usage id; a modifier is 0xe0 for left control through 0xe7 for right GUI.
    pub usage: u8,
    /// Whether it went down or came up.
    pub action: KeyAction,
}

/// How many positions two reports are compared at: eight modifier bits, six release slots and
/// six press slots. Each position yields at most one change, so this bounds the changes too.
const CHANGE_POSITIONS: u8 = 20;

/// **The changes between two reports, one at a time**, as [`changes`] returns them. It holds the
/// two reports and one bit for each position that changed, so it borrows nothing, allocates
/// nothing, and knows its length from the start.
#[derive(Debug, Clone)]
pub struct Changes {
    before: BootReport,
    after: BootReport,
    /// One bit for each position still to yield: bits 0 to 7 are the modifier bits, 8 to 13 the
    /// earlier report's slots (releases) and 14 to 19 the later report's (presses).
    pending: u32,
}

impl Changes {
    /// Whether the key at `position` changed.
    fn has_changed_at(before: &BootReport, after: &BootReport, position: u8) -> bool {
        if position < 8 {
            return (before.modifiers ^ after.modifiers) & (1 << position) != 0;
        }
        // A key goes up when the earlier report holds it and the later one does not, and down
        // the other way round. Only its first slot counts, so a usage a confused device reports
        // twice changes once.
        let (from, to, slot) = if position < 14 {
            (before, after, usize::from(position - 8))
        } else {
            (after, before, usize::from(position - 14))
        };
        let usage = from.keys[slot];
        usage != 0 && !to.holds(usage) && !from.keys[..slot].contains(&usage)
    }

    /// The change at `position`, which [`Changes::has_changed_at`] found changed.
    fn at(&self, position: u8) -> KeyChange {
        if position < 8 {
            let action = if self.after.modifiers & (1 << position) != 0 {
                KeyAction::Press
            } else {
                KeyAction::Release
            };
            return KeyChange {
                usage: FIRST_MODIFIER_USAGE + position,
                action,
            };
        }
        if position < 14 {
            KeyChange {
                usage: self.before.keys[usize::from(position - 8)],
                action: KeyAction::Release,
            }
        } else {
            KeyChange {
                usage: self.after.keys[usize::from(position - 14)],
                action: KeyAction::Press,
            }
        }
    }
}

impl Iterator for Changes {
    type Item = KeyChange;

    fn next(&mut self) -> Option<KeyChange> {
        if self.pending == 0 {
            return None;
        }
        // The lowest pending bit is below twenty, so it fits a u8.
        let position = self.pending.trailing_zeros() as u8;
        self.pending &= self.pending - 1;
        Some(self.at(position))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.pending.count_ones() as usize;
        (left, Some(left))
    }
}

impl ExactSizeIterator for Changes {}

impl core::iter::FusedIterator for Changes {}

/// **What changed from `before` to `after`**, as an iterator of [`KeyChange`]s in the order that
/// types correctly: every modifier change first, in bit order, then the keys released, then the
/// keys pressed. A modifier is its usage, 0xe0 for left control through 0xe7 for right GUI, so
/// every change names a usage of the Keyboard/Keypad page, and [`evdev_code`] maps it to an evdev
/// code. Shift and a letter arriving in one report must reach the keymap shift first, which is why
/// modifiers lead; a key released and another pressed in one report is a roll, and the release
/// first is what a person did. Every usage the device reports is delivered, whether or not
/// [`evdev_code`] knows it; the caller skips what it cannot map.
///
/// At most twenty changes: eight modifiers, six releases, six presses. A caller that wants a
/// callback writes `changes(&before, &after).for_each(...)`.
///
/// # Examples
///
/// Shift and `h` arrive together in one report, so the shift press comes first:
///
/// ```
/// use universal_serial_bus::boot_keyboard::{BootReport, KeyAction, changes, evdev_code};
///
/// # fn main() {
/// #     assert!(example().is_some());
/// # }
/// # fn example() -> Option<()> {
/// let before = BootReport::RELEASED;
/// let after = BootReport::parse(&[0x02, 0, 0x0b, 0, 0, 0, 0, 0])?; // left shift, usage 0x0b (h)
/// let seen: Vec<(u8, KeyAction)> = changes(&before, &after)
///     .map(|change| (change.usage, change.action))
///     .collect();
/// assert_eq!(seen, [(0xe1, KeyAction::Press), (0x0b, KeyAction::Press)]); // LeftShift, then h
/// assert_eq!(evdev_code(0xe1), Some(42)); // KEY_LEFTSHIFT
/// # Some(())
/// # }
/// ```
pub fn changes(before: &BootReport, after: &BootReport) -> Changes {
    let mut pending = 0;
    for position in 0..CHANGE_POSITIONS {
        if Changes::has_changed_at(before, after, position) {
            pending |= 1 << position;
        }
    }
    Changes {
        before: *before,
        after: *after,
        pending,
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;

    fn report(modifiers: u8, keys: &[u8]) -> BootReport {
        let mut b = [0u8; 8];
        b[0] = modifiers;
        b[2..2 + keys.len()].copy_from_slice(keys);
        BootReport::parse(&b).unwrap()
    }

    /// The changes as evdev codes, pressed or not, the way a caller with an evdev keymap reads them.
    fn difference(a: &BootReport, b: &BootReport) -> Vec<(u16, bool)> {
        changes(a, b)
            .filter_map(|change| {
                evdev_code(change.usage).map(|code| (code, change.action == KeyAction::Press))
            })
            .collect()
    }

    /// The changes as the usages the device sent, pressed or not.
    fn usages(a: &BootReport, b: &BootReport) -> Vec<(u8, bool)> {
        changes(a, b)
            .map(|change| (change.usage, change.action == KeyAction::Press))
            .collect()
    }

    /// **`changes` speaks usages**: a modifier is 0xe0 to 0xe7 in bit order, and a usage with no
    /// evdev code is still delivered, so the caller sees it and skips it.
    #[test]
    fn changes_emits_hid_usages_and_modifiers_as_their_own_usages() {
        assert_eq!(
            usages(&BootReport::RELEASED, &report(0x82, &[0x0b])),
            [(0xe1, true), (0xe7, true), (0x0b, true)]
        );
        assert_eq!(
            usages(&report(0x01, &[0x74]), &BootReport::RELEASED),
            [(0xe0, false), (0x74, false)] // 0x74 Execute has no evdev code
        );
    }

    /// The eight modifier rows of [`evdev_code`], and the usages around them that have none.
    #[test]
    fn the_code_lookup_answers_for_the_eight_modifiers_by_their_usage() {
        for (bit, code) in [29, 42, 56, 125, 97, 54, 100, 126].into_iter().enumerate() {
            assert_eq!(evdev_code(0xe0 + bit as u8), Some(code));
        }
        assert_eq!(evdev_code(0xdf), None);
        assert_eq!(evdev_code(0xe8), None);
        assert_eq!(evdev_code(0xff), None);
    }

    /// The table's rows land where the keycaps say: an off-by-one row puts `s` on the `a` key.
    #[test]
    fn the_usage_table_matches_the_keycaps() {
        for (usage, code) in [
            (0x04, 30), // a
            (0x1d, 44), // z
            (0x1e, 2),  // 1
            (0x27, 11), // 0
            (0x28, 28), // enter
            (0x2a, 14), // backspace
            (0x2c, 57), // space
            (0x38, 53), // slash
            (0x4f, 106),
            (0x52, 103), // up
            (0x63, 83),  // keypad dot
            (0x64, 86),  // the ISO key beside the left shift
            (0x73, 194), // F24
            (0x87, 89),  // the Japanese ro key
            (0x8b, 94),  // muhenkan, the table's last row
        ] {
            assert_eq!(evdev_code(usage), Some(code), "usage {usage:#x}");
        }
        assert_eq!(evdev_code(0), None);
        assert_eq!(evdev_code(3), None);
        assert_eq!(evdev_code(0x74), None); // Execute, a hole
        assert_eq!(evdev_code(0x82), None); // a locking key, a hole
        assert_eq!(evdev_code(0x8c), None); // past the table
        assert_eq!(evdev_code(0xff), None);
    }

    /// **The table is the one the Linux cross-check compared.** The table was written from the
    /// Usage Tables and the header, then compared with `hid_keyboard[]` in
    /// `drivers/hid/hid-input.c` at `torvalds/linux@cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a`
    /// (GPL-2.0-or-later). The comparison read that array's entries for usages 0x04 to 0x8b and
    /// 0xe0 to 0xe7 as numbers and compared them with [`evdev_code`], a
    /// hole counting as 0, as an unmapped entry does there. Four of the 144 differ, and the Usage
    /// Tables decide each: 0x74 `Execute` is a hole here and `KEY_OPEN` there; 0x76 `Menu` is
    /// `KEY_MENU` here and `KEY_PROPS` there; 0x77 `Select` is `KEY_SELECT` here and `KEY_FRONT`
    /// there, since an array of bytes cannot hold 353; and 0x86 `Keypad Equal Sign` is
    /// `KEY_KPEQUAL` here and unmapped there. The other 140 agree, including the choices the
    /// module documentation names as this crate's. The crate holds none of that file's text, so
    /// this test pins the result instead: a fingerprint of the 144 numbers. A changed row fails
    /// it, and whoever changes the row reruns the comparison and writes its result here. Where
    /// the two then differ, the Usage Tables decide, unless they are shown wrong.
    #[test]
    fn the_table_matches_its_linux_cross_check() {
        // The fingerprint of the table the comparison above agreed with.
        const FINGERPRINT: u32 = 0x4435_0d80;
        // FNV-1a, 32-bit, over each code's two little-endian bytes in usage order, a hole as 0.
        let mut hash: u32 = 0x811c_9dc5;
        let codes = (0x04..=0x8b)
            .map(|usage| evdev_code(usage).unwrap_or(0))
            .chain((0xe0..=0xe7).map(|usage| evdev_code(usage).unwrap_or(0)));
        for code in codes {
            for byte in code.to_le_bytes() {
                hash = (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193);
            }
        }
        assert_eq!(
            hash, FINGERPRINT,
            "the table changed: rerun the cross-check"
        );
    }

    /// **`echo hello`, typed the way QEMU's `sendkey` presses it**: one key down, then up, eleven
    /// times. What the end-to-end gate types, decoded on the host.
    #[test]
    fn echo_hello_decodes_key_by_key() {
        let mut state = BootReport::RELEASED;
        let mut presses = Vec::new();
        for usage in [
            0x08u8, 0x06, 0x0b, 0x12, 0x2c, 0x0b, 0x08, 0x0f, 0x0f, 0x12, 0x28,
        ] {
            for next in [report(0, &[usage]), BootReport::RELEASED] {
                presses.extend(
                    changes(&state, &next)
                        .filter(|change| change.action == KeyAction::Press)
                        .filter_map(|change| evdev_code(change.usage)),
                );
                state = next;
            }
        }
        // e c h o space h e l l o enter, in evdev codes.
        assert_eq!(presses, [18, 46, 35, 24, 57, 35, 18, 38, 38, 24, 28]);
    }

    #[test]
    fn shift_and_a_letter_together_deliver_shift_first() {
        assert_eq!(
            difference(&BootReport::RELEASED, &report(0x20, &[0x04])), // right shift + a
            [(54, true), (30, true)]
        );
    }

    /// A modifier held across two reports changes nothing, and letting it go is a release.
    #[test]
    fn a_held_modifier_is_quiet_and_its_release_is_a_release() {
        let shift_a = report(0x20, &[0x04]);
        let shift_a_b = report(0x20, &[0x04, 0x05]);
        assert_eq!(difference(&shift_a, &shift_a_b), [(48, true)]);
        assert_eq!(
            difference(&shift_a_b, &report(0, &[0x04, 0x05])),
            [(54, false)]
        );
    }

    /// A roll: `a` still held, `b` pressed, then `a` released while `b` stays.
    #[test]
    fn a_roll_presses_each_key_once_and_releases_it_once() {
        let a = report(0, &[0x04]);
        let a_b = report(0, &[0x04, 0x05]);
        let b = report(0, &[0x05]);
        assert_eq!(difference(&a, &a_b), [(48, true)]);
        // Slot order is not promised, so `b` moving to slot 0 must not read as a new press.
        assert_eq!(difference(&a_b, &b), [(30, false)]);
    }

    #[test]
    fn a_phantom_report_is_refused_rather_than_read_as_a_release() {
        assert_eq!(BootReport::parse(&[0, 0, 1, 1, 1, 1, 1, 1]), None);
        assert_eq!(BootReport::parse(&[0, 0, 0, 0]), None);
    }

    /// The iterator knows how many changes are left, and an exhausted one stays exhausted.
    #[test]
    fn the_changes_iterator_knows_its_length_and_stays_done() {
        let mut all = changes(&report(0xff, &[4, 5, 6, 7, 8, 9]), &BootReport::RELEASED);
        assert_eq!(all.len(), 14);
        assert_eq!(all.by_ref().count(), 14);
        assert_eq!(all.len(), 0);
        assert_eq!(all.next(), None);
        let mut most = changes(
            &report(0x0f, &[4, 5, 6, 7, 8, 9]),
            &report(0xf0, &[10, 11, 12, 13, 14, 15]),
        );
        assert_eq!(most.len(), 20);
        assert_eq!(most.next().map(|change| change.usage), Some(0xe0));
        assert_eq!(most.size_hint(), (19, Some(19)));
        assert_eq!(most.count(), 19);
    }

    /// The constructor refuses what the parser refuses, and reads back what it was given.
    #[test]
    fn a_built_report_is_one_the_parser_would_accept() {
        for error in 1..=3 {
            assert_eq!(BootReport::new(0, [0, 0, error, 0, 0, 0]), None);
        }
        let built = BootReport::new(0x22, [0x04, 0, 0, 0, 0, 0x05]).unwrap();
        assert_eq!(built.modifiers(), 0x22);
        assert_eq!(built.keys(), [0x04, 0, 0, 0, 0, 0x05]);
        assert_eq!(
            Some(built),
            BootReport::parse(&[0x22, 0xff, 0x04, 0, 0, 0, 0, 0x05])
        );
    }

    /// A device that reports one usage in two slots presses it once.
    #[test]
    fn a_duplicated_usage_presses_once() {
        assert_eq!(
            difference(&BootReport::RELEASED, &report(0, &[0x04, 0x04])),
            [(30, true)]
        );
    }

    /// A device that reports one usage in two slots of the earlier report releases it once.
    #[test]
    fn a_duplicated_usage_releases_once() {
        assert_eq!(
            difference(&report(0, &[0x04, 0x04]), &BootReport::RELEASED),
            [(30, false)]
        );
    }
}
