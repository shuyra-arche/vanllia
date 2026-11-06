# Toolchain std bootstrap probe

This focused scenario boots Toolchain's actual ELF as Kernel's initial task,
using Vanllia's existing capsule encoder and bounded process runner. No System,
console, filesystem or other service is needed. It verifies the direct Kernel
startup/memory/termination boundary in supervisor mode; it does **not** verify
System's ELF loader, user-mode deployment or its lifecycle handshake.

Build Toolchain with its `build-std` command, and build the Kernel revision
consumed by that Toolchain. Supply an explicit OpenSBI firmware and its expected
SHA-256. The test is opt-in; it never downloads or substitutes these inputs.

```sh
SHUYRA_TOOLCHAIN_RECEIPT=/absolute/toolchain/build/std.json \
SHUYRA_KERNEL_ELF=/absolute/kernel/target/riscv64gc-unknown-none-elf/release/shuyra \
SHUYRA_KERNEL_REVISION=17059c2922c1084d3b816f54d02226488ddb8298 \
SHUYRA_OPENSBI=/absolute/fw_jump.bin \
SHUYRA_FIRMWARE_SHA256=<sha256-of-the-selected-firmware> \
cargo test --locked --test toolchain -- --ignored --nocapture
```

`SHUYRA_QEMU` can select a `qemu-system-riscv64` executable. Each scenario has a
20-second timeout and must exit QEMU successfully, emit exactly one matching
Kernel exit record, and reach Kernel's all-tasks-exited shutdown. A printed
application marker alone does not pass. The expected records are:

| Fixture | Reason | Kernel note |
| --- | --- | --- |
| `toolchain-std` | `0` | `toolchain: std main returned` |
| `std-exit` | `23` | `toolchain: std process exit` |
| `std-panic` | `0xffffff01` | `toolchain: std abort` |
| `std-oom` | `0xffffff01` | `toolchain: std abort` |

Toolchain owns the programs and their assertions. Vanllia checks their artifact
hashes against the build receipt, encodes capsules and observes actual Kernel
termination. UART/process logs, input hashes, QEMU version and results are saved
under `target/toolchain/<run>/`. The declared Kernel revision records the build
input supplied by the caller; its binary hash records what was actually run.
The Toolchain build receipt is left unchanged; execution evidence is separate.

This narrows the verification gap in #19. It does not close that issue's
assembled-System artifact handoff or deployment acceptance.
