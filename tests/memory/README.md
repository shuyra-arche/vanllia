# Native memory integration

This standalone workspace composes one ABI 11 image from the pinned Kernel API,
Client and Server. It exercises real Tasks and Teams through the public ABI,
including a PCI virtio-blk split Virtqueue behind the RISC-V IOMMU. Production
images keep their default ABI 10 selection. No test imports private Server code.

Run from the repository root with nightly Rust, the RV64 bare-metal target,
QEMU 10.0.x and OpenSBI with DBCN:

```sh
export QEMU_RISCV64=qemu-system-riscv64
export QEMU_OPENSBI=/usr/lib/riscv64-linux-gnu/opensbi/generic/fw_dynamic.bin
python3 tests/memory/run.py
python3 tests/memory/run.py --no-build --capsule-version 10
python3 tests/memory/run.py --profile release
```

The default build uses `--locked` and published exact provider pins. For provider
development only, `--kernel /absolute/path/to/kernel` explicitly patches all
three provider packages; `inputs.json` records that override. Do not commit a
lockfile generated with a local override as the pinned consumer lockfile.

`target/evidence/{debug,release}` retains the Capsule, disk, command, QEMU version,
ELF/firmware/image SHA-256, serial, stderr and elapsed/exit result. The negative
Capsule run has its own `reject-v10` directory. Failure and timeout preserve
evidence. Success requires every scenario marker and final controller shutdown.
Host timeouts bound the whole image; guest deadlines bound polling and joins.

## Scenarios

- Native ABI/Ledger 11, explicit Occupy/Publish/Retract/Vacate, old/future call
  rejection and rejection of an otherwise identical ABI 10 Capsule at startup.
- Real CPU missing faults: repeated event observation, explicit publication,
  route loss and timeout.
- Two Teams exchange 128 messages with two writable Memos; compare 64 messages
  keeping mappings against 64 retracting/publishing the sender's mapping.
  A separate Page is delegated and pulled across Teams without changing PageId.
- A Page Service coordinates Block, a stable maintenance copy, Vacate, Missing,
  staging restore, Push, Allow and fault recovery. A passive Task provides an
  in-memory backing store. This is not a durable disk swap implementation.
- CPU and External retain the same writable queue/data Page for 64 real block
  requests. Readback validates 16 KiB. Used completion does not settle External
  responsibility; reset/drain does, while the CPU mapping remains usable.
  An invalid reply destination tests recovery from the Task's result journal;
  completed retirement replay must not execute twice.

## Trusted platform scope

`platform/device.rs` admits only the paired modern virtio-blk PCI function,
validates its BAR and capabilities, and resolves its requester from the FDT's
IOMMU route. Configuration and IOMMU registers stay private to the platform.
Only the entry Team receives a static RW transport aperture at `0x40000000`.
This boot mapping lasts for the Team lifetime, is not a revocable Pend and must
never be used for device reassignment. It is deliberately outside the RAM
compatibility guarantee. No dynamic MMIO or executable-device support is claimed.

One TCG Hart, 128 MiB RAM and the exact requester share a qualified coherent RAM
fabric. The platform separately qualifies local instruction fetch, enabling the
child Team image only after writer retirement and FENCE.I. SMP, noncoherent RAM,
passthrough, hotplug, ATS/PRI, IOThread and ioeventfd are outside this composition.

The retirement provider accepts only status=0 and queue_enable=0 after Kernel
translation withdrawal/fencing. QEMU's
[`virtio_blk_reset`](https://github.com/qemu/qemu/blob/v10.0.13/hw/block/virtio-blk.c#L1149)
drains outstanding block callbacks and removes queued requests before returning.
An enabled queue retains responsibility even after all used entries are visible.
This source-specific drain evidence must be reviewed before changing QEMU series.

## Evidence limits

Debug and release report raw `PERF` timings and explicit map-operation counts.
These are TCG observations, not physical-device performance claims. No physical
machine is available for this task; hardware validation is recorded as not run.
The integration does not establish production-wide ABI migration or a complete
Swap service. Kernel's `api/memory-m6.md` records provider changes and host checks.

A local pinned run with provider `0fb8f9bd0ae5e4a351eb18d53062b5577312ab3e`,
QEMU 10.0.13 and OpenSBI 1.9 recorded the following (single samples, milliseconds):

| Workload | Debug | Release |
| --- | ---: | ---: |
| IPC, 64 messages retaining mappings | 124.2423 | 13.0006 |
| IPC, 64 messages with sender remapping | 2166.4377 | 75.5906 |
| Virtqueue, 64 block requests retaining mappings | 56.9465 | 10.2309 |

Both profiles passed every scenario. The pinned platform's existing portable
admission tests also passed (55 platform + 3 PCI cases). The Native memory CI
repeats the pinned composition and uploads its own timings and complete inputs;
those values may differ from this local run.
