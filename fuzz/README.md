# Fuzz targets of `universal_serial_bus`

The targets use cargo-fuzz and need a nightly toolchain. From the repository root:

```sh
cargo install --locked cargo-fuzz
cargo +nightly fuzz list
cargo +nightly fuzz build
```

The weekly workflow runs every target for five minutes. To run one by hand, give libFuzzer a scratch directory first, so that what it finds does not land among the committed inputs:

```sh
mkdir -p "$TMPDIR/work"
cargo +nightly fuzz run <target> "$TMPDIR/work" corpus/<target>
```

Each `corpus/<target>` directory holds the inputs a run starts from. A crash input is not committed: it becomes a regression test in the crate, beside the code it broke.

## `universal_serial_bus_configuration` and `universal_serial_bus_boot_report_changes`

A USB device is whatever a stranger plugged in, and every descriptor and report it sends is a byte
string it chose. The configuration target hands one string to the device descriptor parser, the
configuration header parser and the walk to a boot keyboard. It holds the answer to the one
property the walk exists for: an endpoint it names is an interrupt IN endpoint whose packet size
fits a boot report. The crate's Kani harnesses prove the walk total up to 25 bytes in any shape and
41 bytes in one fixed shape, so a longer composite configuration, with lengths and types that
disagree, is what the fuzzer adds.

The report target feeds two eight-byte reports to the parser and, for each pair it accepts, holds
`changes` to at most twenty events and to none for a report set against itself. The harnesses prove
the bound and the order for every pair, so this target checks that the parser's acceptance and the
difference agree.
