#!/usr/bin/env bash
# cargo-qemu-test supplies -nographic before the explicit serial argument.
# Adapt that one runner argument, following Source's existing qtest mechanism.
set -o pipefail
args=()
for arg in "$@"; do
    if [ "$arg" != "-nographic" ]; then args+=("$arg"); fi
done
qemu-system-riscv64 "${args[@]}" 2>&1 | tee "$QEMU_TEE_LOG"
