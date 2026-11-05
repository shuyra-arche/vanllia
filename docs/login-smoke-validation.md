# Post-merge Login and requested Shell smoke

Base main is `c61cd031370ccc99745fbeeef22ee879e4a408fc`. A new detached worktree,
fresh recursive submodule checkouts and empty host/product target directories
rebuilt the CLI and all 17 default runtime executables. Source pins and tracked
files were unchanged. Installed nightly-2026-10-09 and cached dependencies were
used. The clean default build's initrd SHA-256 is
`034a02c1e94842a7f2508f657d764352a6ef49e9666f54af9de75db425efe0f9`;
its Kernel SHA-256 is
`9fd8f80072dd50bc007587c5a17b223f9203e0616adfd2e631835f35533eff9c`.

That main artifact authenticated through the existing public Login fixture and
entered Shell. It lacked structured receipts for the requested instance, so
the host deliberately stopped at the receipt gate before sending the command.
The timeout remains a failed run; it is not counted as acceptance.

This intake pins [System's receipt follow-up](https://github.com/shuyra-arche/system/pull/2)
at `0e899cd9bed908dec4d7c102f78077fd6610ef07` and adds one ignored Rust host test.
The final smoke explicitly selected only Login and its necessary closure:
9 selected Unit templates, 15 runtime executables, with Daemon/RTC excluded.
It adds no test Unit and executes the built artifact unchanged.

Single-core QEMU 10.0.13/OpenSBI 1.6 passed this sequence:

1. All seven ordinary startup instances were ready; Shell was absent.
2. The public fixture authenticated, then System bound/readied requested Shell.
3. Shell evaluated `(+ 314000 159)` and returned numeric value `314159`.
4. EOF ended Shell with native Reap/zero, followed by acknowledged retirement.
5. Login resumed; a second EOF ended Login with native Reap/zero. All eight
   ordinary instances were retired, System completed with zero and VM exited
   zero naturally. No host-forced stop was used.

Final minimum initrd SHA-256 is
`ec1ddf1b1ef573e49da5b7b3fcb6690d98a9277a45e768e80cc8ce110fda5f87`;
the Kernel hash is the clean-build hash above. Firmware SHA-256 is
`8f34d231fde9554e1a3c511bb1efac489da310ab0462504c512994601e12d213`
(273472 bytes). Paths/firmware binaries are not committed. Exact verdicts and
relative local evidence IDs are preserved in [login-smoke-results.json](login-smoke-results.json).

The actual publication module's three regression tests/strict Clippy, the two
existing dynamic-Unit reader tests, RV64 debug check and the final host smoke's
strict Clippy passed. An independent read-only review found no blocker for this
normal live-owner smoke. Owner early-exit can still lose a final receipt after
controller-row pruning and is explicitly unverified; missing evidence fails
closed. Old scenarios and further SMP runs are outside this follow-up.

Reproduce with the commands in [the host test README](../tests/product-lifecycle/README.md#login-and-requested-shell-smoke).
Both follow-up PRs remain Draft; the main receipt gap is not claimed fixed until
the System patch and this pin are integrated.
