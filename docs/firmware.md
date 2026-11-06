# Firmware inputs

The current `vanllia` runner requires `[boot.firmware]` in `vanllia.toml` with
`path` or HTTPS `url`, `sha256` and `bytes`. It checks the input and saves exact
artifact bytes; it does not silently select QEMU's default firmware. Changing
firmware requires rebuilding the saved artifact before running it.

```toml
[boot.firmware]
path = "/absolute/path/to/fw_jump.bin"
sha256 = "REPLACE_WITH_VERIFIED_SHA256"
bytes = 12345 # replace with the actual byte length
```

Use `sha256sum` and `wc -c` to identify your file. Record its source revision,
build configuration, toolchain and notices with any distributed artifact. The
Kernel SBI wrapper is an ABI library, not a firmware image.

No OpenSBI binary is distributed here. The optional
[OpenSBI configuration](../firmware/opensbi/README.md) is not selected by default.
Its source pin is `riscv-software-src/opensbi` at
`bd613dd92113f683052acfb23d9dc8ba60029e0a` (v1.6). Runtime compatibility must be
checked against the selected Kernel load ranges and platform.

## Retained source and notice facts

The unmodified generic build at that revision includes GPL-2.0-marked
`platform/generic/andes/andes_pma.c` and `platform/generic/renesas/rzfive/rzfive.c`,
plus their marked headers. Selected ATCSMU and HTIF files carry BSD-3-Clause
notices. Do not label that complete generic output BSD-2-Clause only. Preserve
the exact source, build configuration, `COPYING.BSD`, `ThirdPartyNotices.md` and
all applicable source-file notices when assessing distribution. The optional
configuration excludes two platform implementations; its detailed notice scope
is recorded with that configuration and is not a blanket firmware license grant.

The former `sqware` firmware at source commit
`94e114614bec53424ac55e640fc80cf19a3e7107` was not copied: its introducing history
did not establish an exact upstream revision, complete build recipe and notice
bundle. A hash identifies bytes, not origin or redistribution rights.
