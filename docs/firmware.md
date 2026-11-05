# Firmware input

Vanllia does not ship firmware. Set `SHUYRA_SBI` to an absolute path to a
user-supplied, regular firmware file before using `scripts/qemu-args.nu`,
`scripts/boot.nu`, `scripts/runner.nu` or `scripts/qtest.nu`. An unset variable,
missing file or directory is an error. There is no implicit QEMU default or
legacy binary fallback.

Record the source, full revision, build command, compiler version, SHA-256 and
applicable license/notice bundle with the artifact. A checksum identifies bytes;
it does not establish their origin or permission to redistribute them. Review
that record before choosing the firmware for runtime acceptance.

## A verifiable upstream alternative

The following recipe selects official OpenSBI **v1.6**, full commit
`bd613dd92113f683052acfb23d9dc8ba60029e0a`. The annotated tag object is
`958c08fa3d21612f6fdc684395011e040e9c8c22`. This is an explicit alternative
to the old firmware. Its runtime equivalence has not been established.

On a Linux host with Git, GNU make, Python 3 and a RISC-V GNU cross toolchain
(including `riscv64-linux-gnu-gcc`, binutils and objcopy):

```sh
git clone https://github.com/riscv-software-src/opensbi.git opensbi
git -C opensbi checkout --detach bd613dd92113f683052acfb23d9dc8ba60029e0a
test "$(git -C opensbi rev-parse HEAD)" = bd613dd92113f683052acfb23d9dc8ba60029e0a
git -C opensbi diff --exit-code
make -C opensbi PLATFORM=generic CROSS_COMPILE=riscv64-linux-gnu- \
  FW_DYNAMIC=n FW_PAYLOAD=n FW_JUMP=y \
  FW_TEXT_START=0x80000000 FW_JUMP_ADDR=0x80200000 \
  FW_JUMP_FDT_ADDR=0x82200000
riscv64-linux-gnu-gcc --version
riscv64-linux-gnu-ld --version
make --version
git -C opensbi rev-parse HEAD
sha256sum opensbi/build/platform/generic/firmware/fw_jump.bin
export SHUYRA_SBI="$(realpath opensbi/build/platform/generic/firmware/fw_jump.bin)"
```

Keep the build log, `.config`, source tree and tool versions with the hash.
The recipe pins source and build parameters; different compiler/binutils
versions can produce different bytes. It is not a bit-for-bit reproducibility
claim. Do not use `make run` during build preparation: that starts QEMU.

`FW_JUMP_ADDR=0x80200000` matches the selected Kernel's
[`link.x`](../components/kernel/link.x). The firmware lives at `0x80000000`;
`0x82200000` explicitly selects generic-platform's FDT destination. Check the
actual Kernel ELF load ranges for overlap before runtime use. QEMU's `-kernel`
and `-initrd` remain separate inputs; no Kernel or product is embedded in this
firmware. No switch to `fw_dynamic`, a payload image, or `-bios default` is made
implicitly. See upstream [FW_JUMP contract](https://github.com/riscv-software-src/opensbi/blob/bd613dd92113f683052acfb23d9dc8ba60029e0a/docs/firmware/fw_jump.md)
and [generic defaults](https://github.com/riscv-software-src/opensbi/blob/bd613dd92113f683052acfb23d9dc8ba60029e0a/platform/generic/objects.mk).

The official recipe was built locally; see the
[build and license findings](integration-build-validation.md). It produces a
traceable alternative artifact, with no claim of boot equivalence.

OpenSBI's [README](https://github.com/riscv-software-src/opensbi/blob/bd613dd92113f683052acfb23d9dc8ba60029e0a/README.md)
states BSD-2-Clause, with reused libfdt code selected under its BSD option.
That summary does not describe every selected file. At this exact revision the
default generic build also compiles GPL-2.0-marked
[`andes_pma.c`](https://github.com/riscv-software-src/opensbi/blob/bd613dd92113f683052acfb23d9dc8ba60029e0a/platform/generic/andes/andes_pma.c)
and [`rzfive.c`](https://github.com/riscv-software-src/opensbi/blob/bd613dd92113f683052acfb23d9dc8ba60029e0a/platform/generic/renesas/rzfive/rzfive.c),
plus their GPL-marked headers; their functions are present in the generated ELF.
Selected `atcsmu` and `htif` files carry BSD-3-Clause notices.

Do not describe or redistribute the resulting generic firmware as BSD-2-Clause
only. Retain the complete exact source, build/configuration record,
`COPYING.BSD`, `ThirdPartyNotices.md`, and each selected file's copyright and
license notices. A distribution needs the applicable GPL source/license and
BSD-3-Clause material as well as the BSD-2-Clause/libfdt notices; that complete
distribution assessment is still open. This default-generic result remains
intact. A separate [optional configuration candidate](../firmware/opensbi/README.md)
uses official switches to exclude two platforms, with two-build repeatability
and source/link evidence; it is not adopted by a boot entry. Alternatively,
supply a separately documented firmware artifact with its own verified source
and notice record.

No firmware binary is added to Vanllia or selected as its default. This work
does not add a blanket license to either repository.

## Old artifact investigation

Frozen source: `anreonyr/sqware` at
`94e114614bec53424ac55e640fc80cf19a3e7107`. Its `SBI.bin` has SHA-256
`c53d58aa9ab17f95fa1af159f0bf05fad5c5622ebc0f1084463435fa6e5752b6`.
The accompanying ELF contains `fw_jump.o`, an OpenSBI banner, and a GCC 15.1.0
comment. The binary's introducing history (`be0a304c`, `rename`) supplies no
verified upstream revision, complete build recipe, or applicable notice bundle.
These observations do not identify an exact OpenSBI release. Neither `SBI.bin`
nor `SBI.elf` is copied into the new entry.

`shuyra-arche/sbi` is the Kernel's SBI ABI wrapper. It is not a replacement
firmware image. Choosing an upstream build or another user-supplied artifact
requires a new, explicitly recorded runtime check; old SMP evidence does not
apply automatically.
