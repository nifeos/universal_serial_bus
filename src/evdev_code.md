Most names match on their face (`F1` is `KEY_F1`); where they do not, the Usage Tables decide:

- 0x2a `DELETE (Backspace)` is `KEY_BACKSPACE` and 0x4c `Delete Forward` is `KEY_DELETE`, by
  footnotes 4 and 8: the first backs up the cursor, the second deletes in place.
- 0xe3 and 0xe7, `Left GUI` and `Right GUI`, are `KEY_LEFTMETA` and `KEY_RIGHTMETA`, because
  footnotes 33 and 34 list the Sun Meta key among the GUI keys.
- 0x64 `Non-US \ and |` is `KEY_102ND`, the 102nd key beside the left Shift by footnote 10, and
  0x65 `Application` is `KEY_COMPOSE`, named the Windows key and Compose by footnote 11.
- 0x87 to 0x8b, `International1` to `International5`, are `KEY_RO`, `KEY_KATAKANAHIRAGANA`,
  `KEY_YEN`, `KEY_HENKAN` and `KEY_MUHENKAN`, the legends the Usage Tables' Japanese table
  gives them (ろ, ひらがな and カタカナ, ¥, 変換, 無変換).

# This crate's choices

The sources leave six things open, and this crate chose them:

- **Usage 0x32 (`Non-US # and ~`) is `KEY_BACKSLASH`.** The header has no key for it. The
  usage sits at Advanced Technology (AT) 101-key position 42, beside Return, and footnote 5 gives `\` and `|` as its
  United States (US) legends, so a US layout reads it as the backslash key.
- **Usage 0x46 (`PrintScreen`) is `KEY_SYSRQ`.** The header's `KEY_PRINT` carries the comment
  `AC Print`, the Consumer page's print command, not a keyboard key. System Request is the
  function a personal computer (PC) keyboard puts on the Print Screen keycap.
- **A usage named for two legends takes the first.** `Keypad Num Lock and Clear` (0x53) is
  `KEY_NUMLOCK`, and `Keypad 1 and End` (0x59) through `Keypad . and Delete` (0x63) are the
  keypad keys `KEY_KP1` through `KEY_KPDOT`. The second legend is what the key does in another
  mode, which the keymap decides, and the Usage Tables give Clear its own usages (0x9c, 0xd8).
- **A usage whose only match is another page's key takes it.** `Power`, `Help`, `Stop`, `Undo`,
  `Cut`, `Copy`, `Paste` and `Find` (0x66, 0x75, 0x78, 0x7a to 0x7e) match only header names
  whose comments give a System Control, Application Launch or Consumer usage, and no keyboard
  key competes for them. `Keypad Equal Sign` (0x86, footnote 14) is `KEY_KPEQUAL`, as is
  `Keypad =` (0x67).
- **0x74 `Execute` has no code.** No header name matches it.
- **0x82 to 0x84, the locking Caps, Num and Scroll Lock, have no code.** Footnote 12 says each
  is held in the report while the lock is on, so a press and a release would toggle a keymap
  twice for one change. Keyboards are expected to send the non-locking keys (0x39, 0x53, 0x47).
