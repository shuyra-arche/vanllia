# Native verification

This standalone Cargo workspace is a host utility. It is outside the product
workspace and is never built as an RV64 guest. Run its host contracts with:

```sh
cargo test --locked --manifest-path tests/runner/Cargo.toml --target x86_64-unknown-linux-gnu
```

`verify native` owns the current memory/IPC/Swap/Virtqueue composition, including
ABI rejection; see [the memory guide](../memory/README.md). `verify versions`
checks the selected product's coordinated Kernel manifests, locks and gitlink.
Other Vanllia tooling has not been migrated by this change.

`verify system --kernel PATH --fixture SYSTEM --twine TWINE --output NEW` owns
the native System/Twine runtime regression. System is pinned to
`7e4f9dc19bdaf4a0e95ff3b557fdc8959b8547d7` and Twine to
`7c75c62d15d173032d99fed54ca7fd00337d787b`; it executes release on 1/4 Harts.
Neither command claims complete System or physical hardware acceptance.

The library supplies one ELF-to-Capsule packager and bounded process execution
for these commands and provider-owned Kernel probes. Scenario assertions stay
with their owner. Output directories must be new. Tools preserve process output
before checking acceptance; success requires a normal exit plus complete
scenario evidence. The whole process group is killed and reaped at a deadline.
Input checkouts are read through committed Git archives. Working-tree edits are
not silently included. Every build preserves its original and resolved lock,
commands, compiler/emulator/firmware identity and image hashes. Host contracts
cover large output, deadlines after success output, signals, incomplete output
and malformed ELF inputs.

Integration order for Kernel #199: publish this runner commit, update Kernel's
runner dependency and fixture to that exact commit, then validate the assembled
revision set. No moving branch pins and no implicit claim that unrelated
Vanllia or System scripting has been removed.
