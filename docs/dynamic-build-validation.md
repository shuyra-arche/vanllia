# Dynamic build migration validation

This record describes the earlier dynamic-build and facade-alignment revisions.
For the later Kernel API/Client consumer pins and fresh build/runtime receipts,
see [SDK consumer validation](sdk-consumer-validation.md).

The product entry is `vanllia.toml` and the independent `vanllia` CLI. The root
has no Cargo composition, bootstrap, image adapter or Nushell build/runner entry.
Provider-owned TOML supplies Unit policy and actual Cargo bin identity; the
Deployment is an in-memory selection, never a file consumed by System.

Validation used installed nightly-2026-10-09 and LLVM tools. Historical provider
toolchain pins remain unchanged. All gitlinks and SDK source locks refer to
published commits; development path-patched locks were not committed as release
locks. The CLI's Cargo overlays and their generated locks live under `.vanllia`.

## Actual release artifacts

| Selection | Unit records | Runtime ELF files | Initrd SHA-256 |
| --- | ---: | ---: | --- |
| Default Login + Daemon closure | 11 | 17 | `5dedec40ece34f31437844080ed7778ccfcb04da14c871a76bc876fa679827a4` |
| Pipe roundtrip | 3 | 3 | `9d35619c8f670552b8b9296b3e427772b17f8329197ffac94100211811c3dd20` |

The Kernel is a separate built boot artifact. Debug executables remain on the
host; stripped runtime images and binary Unit records are packaged once. A Rust
inspector using the public bundle/record readers validated the default artifact:
11 identities, 17 files, correct boot image, and Shell as the sole requested
template. System-generated machine inventory is not a fake file in initrd.
The minimal artifact is checked separately for exactly System, Pipe and the
new Rust test Unit. `vanllia test -- --list` discovers only `pipe.roundtrip`.

```sh
vanllia fetch
vanllia check
vanllia plan
vanllia build
vanllia build --unit test.pipe-roundtrip
vanllia test -- --list
```

The full product compiled 17 real Cargo executables without a central image
table. Actual integration caught and fixed two production/dev SDK alias clashes:
new Shell/Login declaration tests now live in isolated workspaces, preserving
production locks. Packaging also caught the difference between initrd files and
resources System generates from the boot ledger; the System package now owns
that metadata. A deterministic fork test reproduced and fixed delayed release
of inherited build-lock descriptors. Earlier failed attempts remain failures,
not passing guest evidence.

## Scoped host verification

- CLI: 38 Rust tests, strict Clippy and formatting passed. Two tests build real
  RV64 artifacts; other cases cover discovery, source locks, overlays, workspace
  editor configuration and typed native lifecycle verdicts.
- Host library: 22 inline and 8 retained integration cases passed, including
  timeout, native nonzero exit, cleanup failures and inherited lock release.
- System: actual RV64 executable and two new host tests of the real catalog,
  binary decoder and startup selector passed. Existing server warnings remain.
- Each provider's new declaration contract tests were reviewed independently
  against public exact Unit revisions and formal locks. System, Driver and
  Session intake PRs retain their stated verification boundaries.
- Kernel event rings and hot-path writes are removed. RV64 default/semihosting
  checks and the native crash-export test passed; exceptions and backtraces stay.

## Runtime boundary

The minimal three-Unit Rust Pipe case passed single-core QEMU 10.0.13 (Debian
1:10.0.13+ds-0+deb13u1) with OpenSBI 1.6. Native Reap/zero exit, explicit endpoint
revocation, acknowledged retirement and VM exit zero all passed. `verdict.json`
binds that result to the exact input hashes. Kernel SHA-256 is
`563948be2580411cbe2812ac114e31f215c3e54c8a5e407bb76080e201f477ae`;
installed firmware SHA-256 is
`8f34d231fde9554e1a3c511bb1efac489da310ab0462504c512994601e12d213`
(273472 bytes). The machine-specific firmware path was used only for this local
run and is not committed. The first runs exposed UART CRLF framing and an
incorrect test cleanup order; both were fixed and the final case also exercises
partial I/O with a payload larger than capacity. No failure was converted to a pass.
The unchanged full product artifact also passed single-core runtime acceptance.
All nine ordinary startup instances became ready; the Shell template stayed
uninstantiated. Terminal EOF then ended Login with native Reap/zero, all nine
instances retired, System completed with zero and QEMU shut down naturally.
No host-forced stop was used. The reusable
[Rust acceptance test](../tests/product-lifecycle/README.md) passed from its
formal lock and remains explicitly ignored unless runtime execution is requested.
An earlier observer stopped the VM after readiness only; that observation is
kept distinct from the later natural-exit/reclamation verdict. Initial EOF test
attempts failed because the socket capture was not drained and input preceded
Login's terminal attachment; neither failure is counted as a pass.
The same unchanged product artifact and minimal Pipe case also passed at 2 and
4 CPUs, using the same firmware and Kernel hashes. Every run required native
zero exit, acknowledged retirement, System completion and VM exit zero. Product
EOF runs used no host-forced stop. The six 1/2/4-CPU verdicts are summarized in
[runtime-results.json](runtime-results.json); raw serial/process evidence remains
under the recorded local run IDs.

The table now identifies the current facade-aligned artifacts. Env's default
branch moved its ABI/ledger definitions into Kernel while this work was active.
Env PR #2 now includes that base through an ordinary merge commit: `env` stays
the facade, the Unit bundle belongs to `env-initrd`, and its capsule reader is a
direct re-export of Kernel's canonical reader. No deleted ABI sources were
restored. The root pins Env `ee935707`, Wire `08293d2d` and Mold `8e8b1c2b` together;
selecting only the new Env had correctly failed on incompatible wire traits.
The CLI/System dependency pins and formal locks were updated too.

The published-source product rebuilt all 17 images, and both current selections
passed fresh 1/2/4-CPU exit/reclamation acceptance. Their Kernel SHA-256 is
`4427cefdaa834c30e35f96188fae4ba0ec077d5fdce6b023accb0e5366b0f82e`.
`runtime-results.json` separates these six current results from six historical
pre-alignment runs (whose Kernel hash above remains recorded). Env passed all
52 host tests, RV64 workspace checking and scoped strict Clippy; its whole-test
Clippy still reports two existing constant-chunk diagnostics in `tests/memory.rs`.
The CLI's 38 tests/strict Clippy and System's two reader tests were rerun.

A separate bounded [Rust syscall benchmark](../tests/syscall-cost/README.md)
compared Kernel `3aa0a155f825fbf377fc8d04009472ca67cbbf8c` (event rings present)
with `44e33a6ee28d86eeeabe61c225f4176c5ed64c59` (removed). Both were built with the
same release profile, target flags, toolchain and byte-identical Cargo lock;
the benchmark initrd and installed firmware were identical across all ten boots.
This comparison predates the facade alignment and keeps its own exact inputs;
it is not claimed as a new timing sample of the current product artifact.
The only packaged Units were System and the benchmark. Each boot also passed
native exit and reclamation checks independently of its measurement frame.

| 100,000 SelfId calls, single-core QEMU | Median | Minimum–maximum, 5 samples |
| --- | ---: | ---: |
| Event rings present | 2267.129 ms | 2214.871–2283.292 ms |
| Event rings removed | 2155.845 ms | 2130.424–2219.071 ms |

The median decreased 4.91% in this fixed workload. This is a small paired QEMU
microbenchmark, not a general throughput or hardware performance claim. Timing
excludes boot, measurement output and teardown. Authenticated Shell creation and
old scenarios remain outside this runtime check.
The new runner is managed and records headless serial evidence.
Boot firmware must be configured with an explicit file/URL, SHA-256 and length;
these build artifacts do not bundle a firmware binary. No firmware was silently
substituted. The old scenarios and tests are not migrated; retained composition
test sources, evidence scripts and earlier documents are historical material.

## Review chain

These PRs were prepared as Drafts and validated against the pinned commits above.
The authorized merge uses merge commits into each repository's default branch,
retaining those exact commits for source locks and submodules:

- [CLI and host builder](https://github.com/shuyra-arche/image-builder/pull/2),
  [System](https://github.com/shuyra-arche/system/pull/1),
  [Unit records](https://github.com/shuyra-arche/unit/pull/1),
  [initrd format](https://github.com/shuyra-arche/env/pull/2),
  [kernel event log removal](https://github.com/shuyra-arche/kernel/pull/4).
- [Pipe](https://github.com/shuyra-arche/pipe/pull/1),
  [Shell](https://github.com/shuyra-arche/shell/pull/8),
  [Daemon](https://github.com/shuyra-arche/daemon/pull/2).
- [Driver intake](https://github.com/shuyra-arche/driver/pull/6):
  [Hub](https://github.com/shuyra-arche/driver-hub/pull/1),
  [Router](https://github.com/shuyra-arche/driver-router/pull/3),
  [UART](https://github.com/shuyra-arche/driver-uart/pull/2),
  [RTC](https://github.com/shuyra-arche/driver-rtc/pull/3).
- [Session intake](https://github.com/shuyra-arche/session/pull/4):
  [Login](https://github.com/shuyra-arche/session-login/pull/2),
  [Terminal](https://github.com/shuyra-arche/session-terminal/pull/3),
  [Account](https://github.com/shuyra-arche/session-account/pull/1).
