# Optional OpenSBI configuration candidate

This is a build/configuration proposal, not a selected runtime firmware.
`SHUYRA_SBI` stays mandatory and no boot command or default changes.

`no-andes-renesas_defconfig` starts from official OpenSBI v1.6 generic
`platform/generic/configs/defconfig` at
`bd613dd92113f683052acfb23d9dc8ba60029e0a`. Its only two input changes disable
`PLATFORM_ANDES_AE350` and `PLATFORM_RENESAS_RZFIVE`. These are official
[Kconfig selectors](https://github.com/riscv-software-src/opensbi/blob/bd613dd92113f683052acfb23d9dc8ba60029e0a/platform/generic/Kconfig),
consumed through upstream's documented `PLATFORM_DEFCONFIG` interface.
No upstream C, assembly, linker script or Makefile is patched.

This is a narrow exclusion of two platform implementations, not a minimal
QEMU-only firmware. Other platform overrides, HTIF, serial and generic FDT
drivers are retained where their dependencies allow them. Andes/Renesas support
is intentionally removed and has not been tested on any machine.

## Build recipe and evidence

Use a fresh source/output directory for each build. From the Vanllia root:

```sh
vanllia_root="$(pwd)"
git clone https://github.com/riscv-software-src/opensbi.git /absolute/opensbi-src
opensbi_src="$(realpath /absolute/opensbi-src)"
git -C "$opensbi_src" checkout --detach bd613dd92113f683052acfb23d9dc8ba60029e0a
test "$(git -C "$opensbi_src" rev-parse HEAD)" = bd613dd92113f683052acfb23d9dc8ba60029e0a
cp "$vanllia_root/firmware/opensbi/no-andes-renesas_defconfig" \
  "$opensbi_src/platform/generic/configs/no-andes-renesas_defconfig"
export SOURCE_DATE_EPOCH=1735034298
make -C "$opensbi_src" -j4 V=1 PLATFORM=generic \
  PLATFORM_DEFCONFIG=no-andes-renesas_defconfig \
  CROSS_COMPILE=riscv64-linux-gnu- \
  CC="riscv64-linux-gnu-gcc -ffile-prefix-map=$opensbi_src=/usr/src/opensbi -fdebug-prefix-map=$opensbi_src=/usr/src/opensbi" \
  BUILD_INFO=n OPENSBI_VERSION_GIT=v1.6 \
  FW_DYNAMIC=n FW_PAYLOAD=n FW_JUMP=y \
  FW_TEXT_START=0x80000000 FW_JUMP_ADDR=0x80200000 \
  FW_JUMP_FDT_ADDR=0x82200000
sha256sum "$opensbi_src/build/platform/generic/firmware/fw_jump.bin" \
  "$opensbi_src/build/platform/generic/firmware/fw_jump.elf"
sha256sum "$opensbi_src/build/platform/generic/kconfig/.config"
```

Upstream writes the resolved configuration to
`build/platform/generic/kconfig/.config`; inspect that generated file to audit
the seven disabled symbols below. It is reproduced from the supplied input and
pinned Kconfig, rather than kept as a second configuration authority here.
The fixed epoch above is this upstream commit's `git show -s --format=%ct`
value. The build-info timestamp branch is disabled by `BUILD_INFO=n`.

GCC must support both prefix-map flags. They normalize debug paths and
`__FILE__` diagnostics; this is an explicit diagnostic change from the earlier
default build. `BUILD_INFO=n` keeps timestamp/compiler banners off, and the
version string is explicitly fixed. The cross compiler and its libraries must
already be available; an extracted toolchain also needs its library directory
on `LD_LIBRARY_PATH` as recorded in the earlier build evidence.

On 2026-11-05, two independent source/output directories on the same Linux
x86_64 host used GCC Debian 14.2.0-19, binutils 2.44 and Make 4.4.1. Both fresh
builds succeeded with identical resolved configuration, raw firmware and ELF:

| Input or artifact | SHA-256 |
|---|---|
| Input defconfig | `67bc1700bc9a55ec230a38d666bdf4f5dce8bf9fbefa90de7a5d3a42246c9b43` |
| Resolved `.config` | `f3965e9cdc9925225a5c7f5ba7ee12ec3331a68e2dff8abb7d4847fefd32a7bb` |
| 272,808-byte `fw_jump.bin` | `fa4e79f13465a25a25bb5d7deb966fee7037a8d865728082a091175627305fe4` |
| `fw_jump.elf` | `d3a8d982658a9796499b653b401528d0bb3396a3c83df8e30e01e921914e5a1e` |

This demonstrates repeatable bytes for those two fixed-toolchain builds, not
cross-toolchain, cross-host or runtime reproducibility. The prior unmodified
default-generic build has a different hash and remains separate evidence.

## Resolved changes and license facts

The two input changes resolve to seven disabled symbols: the two platform
selectors plus `ANDES_PMA`, `ANDES_SBI`, `ANDES_PMU`, `SYS_ATCSMU` and
`FDT_RESET_ATCWDT200`. The last is an Andes watchdog reset driver depending on
`SYS_ATCSMU`; its requested default cannot remain enabled after that dependency
is removed. Other reset paths, including HTIF and syscon, remain enabled.

Both compiler dependency inventories contain 253 upstream source/header files.
They exclude the four previously selected GPL-2.0-marked Andes/Renesas files.
The corresponding `andes_pma_setup_regions` and `renesas_rzfive` symbols are
absent from both ELFs. This is source/link evidence, not a license verdict.

`lib/utils/sys/htif.c` and its header remain selected and BSD-3-Clause marked;
`htif_serial_init` remains in the ELF. BSD-2-Clause and libfdt dual-license
notices also remain. Five selected files have no individual SPDX identifier;
their applicable repository/per-file terms still need confirmation before a
release assessment. A missing identifier does not itself establish an absence
of permission. Do not call this firmware BSD-2-Clause-only or legally cleared.

An internal distribution-material draft was prepared outside Vanllia with:

- the complete unmodified official source archive, including unselected
  GPL-marked files; the supplied defconfig and resolved configuration;
- upstream `COPYING.BSD`, `ThirdPartyNotices.md` and complete copies of every
  selected source/header with original notices;
- canonical BSD-3-Clause and GPL-2.0-only text from official SPDX data v3.26.0,
  commit `8a04f0303d95c4f932210b118588c662665bd383`;
- both build commands, tool/package hashes, configuration/artifact hashes and
  per-file source/license inventory.

No firmware binary is included in that draft or committed to Vanllia. The GPL
text is retained because the full source archive still contains those files.
These are review materials; selecting applicable license terms, satisfying
distribution conditions and runtime acceptance remain separate work. No source
is relicensed and no legal approval is inferred from a filename or hash.
