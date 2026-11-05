# Local product, Kernel and firmware build evidence

Recorded 2026-11-05 against integration code head
`29d51e2c0406b6cf3b03e2344ae941089626e703`, tree
`f31602ebe41b7d3428836da76a7b90be0a791a93`. A documentation-only follow-up
records these results; production code, locks, provider pins and test sources
remain identical to that tested tree.

Live Draft #6/#7/#8 heads still matched `3003ee3...`, `9d540b3...` and
`5ebafc8...`, all based on `2643ee792b9fc791e24bbfc3a8efedc1f83fd69a`.
All three full source heads remain ancestors of the integration. No remote
branch or PR is changed by this validation.

## Product and Kernel

Linux x86_64, Python 3.12.14, Nushell 0.107.0, repository toolchain
`nightly-2026-09-04` (`rustc 1.100.0-nightly`, `a69a63265`, 2026-09-03):

```sh
rustup component add llvm-tools-preview --toolchain nightly-2026-09-04
export PATH="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin:$PATH"
nu scripts/build.nu release
```

The actual Vanllia entry completed with exit 0 in 238.374 seconds. Product and
Kernel output directories did not exist before the run; Git/Cargo dependency
caches and earlier host-test artifacts were already present. This is a local
build at the combined code tree, not an empty-cache reproducibility claim.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `target/product/release/initrd.img` | 2,912,256 | `81ebe1a1767dcd2ac6804b5629fbb0a0d2e9c808794caf5b4c1b22d476f3b0ea` |
| `target/kernel/riscv64gc-unknown-none-elf/release/shuyra` | 1,886,976 | `8f9f444a61a1d6d0d33626921727dd3260801e5f8cc8f3a61b30073e86968e64` |

The entry's stdout was one JSON artifact record containing the actual initrd
and Kernel paths. It built release Kernel with `semihosting`. Kernel emitted
ten compiler warnings and no compilation error; no warning fix was attempted.

The initrd contains exactly the retained 17 `(image, kind)` entries, with only
`system` marked Supervisor. An external host audit included the pinned System
provider's actual `loader/image/elf.rs` and the selected `env@d3000253` APIs:
all 17 packed ELFs pass the real loader parser and the 16 MiB image-size bound;
the actual manifest and bootstrap capsule parsers also pass. The bootstrap
capsule entry matches the System ELF entry `0x9eb7e`. This audit is scratch build
tooling, not a new repository test suite or a replacement for any old assertion.

Kernel is RV64 ET_EXEC, entry `0x80200000`. Its highest static PT_LOAD end is
`0x802f7154`, below the explicit FDT address `0x82200000`. This is a static
range check, not proof of runtime placement or memory safety.

Root, selected provider and Kernel locks remain unchanged. The existing
320 Python host cases and 5 new plus 6 retained Rust product tests are unchanged;
their earlier passing runs are retained evidence, not reruns in this step.

## Official firmware alternative

Official OpenSBI v1.6 tag object
`958c08fa3d21612f6fdc684395011e040e9c8c22` resolves to source commit
`bd613dd92113f683052acfb23d9dc8ba60029e0a`. The clean source was built with the
exact FW_JUMP recipe in [firmware.md](firmware.md), adding only `-j4 V=1` for
parallelism and command logging. No dynamic/payload image was generated.

Cross GCC: Debian 14.2.0-19; binutils: 2.44; GNU Make: 4.4.1. Cross-tool packages
were extracted under an isolated writable prefix. Initial snapshot downloads
returned HTTP 503/504; the missing packages were fetched from Debian's official
mirror and verified against SHA-256 values from the signed snapshot index.
The extracted binutils also require that prefix's `usr/lib/x86_64-linux-gnu`
in `LD_LIBRARY_PATH`; a first linker probe failed before this was configured.
The successful build uses the unmodified source and documented build settings.

`fw_jump.bin` is 273,432 bytes, SHA-256
`3177f1b0a965fcd43386de2cca56544af882ae4cc67ecd17e86db34b2abd295f`.
Its ELF entry is `0x80000000`, highest static PT_LOAD end `0x80045d90`, and
the actual `_jump_addr` bytes encode `0x80200000`. The command explicitly sets
FDT destination `0x82200000`. Static firmware/Kernel load ranges do not overlap.
Tool versions, configuration, source archive, build log, per-file notices and
artifact hashes were retained outside the repository. Only one successful
firmware build was performed; no bit-for-bit reproducibility proof is claimed.

The default generic firmware includes more than the README's BSD-2-Clause
summary: compiled GPL-2.0 `andes_pma.c` and `rzfive.c` functions remain in the
ELF, and selected files also carry BSD-3-Clause notices. The source/archive and
per-file inventory support a complete-source distribution path, but do not
constitute a completed distribution/license assessment. The remaining
[firmware notice requirements](firmware.md) must be resolved before shipping a
binary. No source is relicensed and no platform is silently removed.

## Remaining boundaries

No QEMU, SMP, firmware execution or guest acceptance was run. Neither the new
firmware's behavior nor its equivalence to the unknown legacy SBI artifact is
proven. `SHUYRA_SBI` remains an explicit external input; no default changed.
The old binary's upstream/build/notice provenance remains unresolved.

No merge was attempted, including through an alternative path. The earlier
merge denial is not overridden by a successful build. No PR, remote update,
history rewrite, squash or archive operation was performed. Dev-only legacy
fixtures, historical checkout and pending guest-scene dependencies still
prevent claiming whole-repository independence or archive readiness.
