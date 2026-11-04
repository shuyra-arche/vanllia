# Product build and boot entry migration

The transport intake described here is the #6 baseline. Its initial copied test
organization is superseded by [acceptance-ownership.md](acceptance-ownership.md):
runtime, evidence verdicts, host fixtures and host cases now have separate
owners, with ordinary package imports and one host discovery command.

Base: Vanllia Draft #5, `2643ee792b9fc791e24bbfc3a8efedc1f83fd69a`.
Legacy evidence source: `anreonyr/sqware`,
`94e114614bec53424ac55e640fc80cf19a3e7107`.
Kernel gitlink: `3aa0a155f825fbf377fc8d04009472ca67cbbf8c`.

This slice moves boot/acceptance transport and Python host checkers into
Vanllia. `scripts/build.nu` orchestrates the existing product image adapter and
Kernel Cargo build, selects their actual output artifacts and emits a JSON
record. The product adapter remains the only image build mechanism. Existing
17-image composition, locks, gitlinks, declarations and deployment order are
unchanged.

The copied Nu scripts retain the old normal-boot observer, buffered stdin,
serial-only stdio routing, board/runner argument split, QEMU environment
variables, EOF feeding, trace filenames and final acceptance checks. Deliberate
changes are the new image command (`cargo image build SCENE PROFILE`), relocated
host driver paths, mandatory explicit firmware, validation of regular artifact
files, and an early gate for image scenes not yet migrated. The legacy
`sqware-diagnose.jsonl` filename is a Kernel protocol fact and remains unchanged.

A host transport test exposed an inherited normal-runner bug on Nushell 0.107:
reading `LAST_EXIT_CODE` after `try` observed 0 when the boot process returned 7.
The observer now captures the process result with `complete` while a raw `tee`
keeps live terminal output. Nonzero runs retain their console evidence instead
of being mislabeled normal. The observer still returns normally after reporting
the result; it does not become an acceptance test. The standalone Shell idle
driver now takes its Kernel path from explicit `SHUYRA_KERNEL` rather than an old
hard-coded target path.

The source blob manifest preserves reviewable provenance. Original history is
accessible through the full immutable source commit; it is not rewritten or
deleted. No legacy firmware binary is added. See [firmware findings](firmware.md)
for the upstream source recipe and unresolved old-binary provenance.

## Remaining dependencies

This is build/run **preparation**. No QEMU, SMP or guest acceptance was run for
this tree. Host checks cannot prove that the latest product or alternative
firmware boots.

The current product still builds `shell` and `cat` from `legacy/sqware` and
consumes the old Shell declaration. The root dev equivalence test and legacy
aggregate retain further fixtures/APIs. Those are independent product/provider
migrations. The old non-product scene deployments and guest fixture programs
have not been moved. The complete list and gate are in
[`acceptance/README.md`](../acceptance/README.md).

A future clean checkout can use these transport scripts and their host tests
without the legacy checkout. **A complete current product build still cannot.**
Remove the remaining product/fixture references, migrate required scenario
providers, complete a clean-checkout build and obtain new runtime evidence before
claiming independence or archiving the old repository. This PR neither merges
the stacked product branches nor archives any repository.
