# Changelog

Dates are UTC. One line per release, what changed and why, so a reader of the diff has the
sentence beside it.

## Unreleased

- First version.
- `Refusal`, its struct variants, `Speed`, `DeviceDescriptor`, `ConfigurationHeader` and
  `BootKeyboard` are non-exhaustive, so a later release can add a variant or a field without
  breaking a caller.
- `BootReport`'s fields are private behind `modifiers()` and `keys()`, and `BootReport::new`
  refuses what the parser refuses, so no caller can build a phantom report.
- `changes` returns an iterator of `KeyChange` values, each a usage and a `KeyAction`, in place of
  a callback of a bare `u8` and `bool`.
- `SetupPacket::request_type` is a `RequestType`, built from a `Direction`, a `RequestKind` and a
  `Recipient`, so no caller can send a reserved value.
- A setup packet turns into its eight wire bytes with `to_le_bytes` and back with
  `SetupPacket::parse`.
- `DeviceDescriptor` carries the subclass, protocol, device release and three string indexes, and
  `ConfigurationHeader` the attributes and maximum power code.
- `Descriptors` walks a configuration one descriptor at a time, and `find_boot_keyboard` walks with
  it, so a caller looking for a mouse or a hub reuses the proven walk.
- Every fallible function documents its errors, every public function has an example, and the
  README is the crate page.
- The package ships only the source, the README, the changelog and the licenses.

## BUGS

- The template's lag check guards the CI pin, not this file; a change that matters to a user of
  the crate belongs in a release line here.
