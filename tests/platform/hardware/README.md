# Exclusive QEMU hardware fixture

This no_std test-only package owns the M-mode startup, UART/reset MMIO, firmware
selection and EDU PCI/device operations used by Kernel's low-level register suite.
It imports the same production `platform-riscv` firmware decoder as the normal
boot image, with its image feature disabled. It has no Kernel Server dependency.
The guest retains Kernel's actual Page, publication, domain and command-engine
implementations; the fixture only supplies the machine and emulated endpoint.

Kernel pins this package by full Vanllia commit in the standalone guest manifest.
It must never become a production Kernel dependency. The fixture applies only to
coherent QEMU virt, immutable routing and emulated EDU, with no passthrough/ATS/PRI.
Actual Task death is tested by the real Kernel Cell `io-exit` image; low-level
closure scenarios certify only the retained hardware target's behavior.

Host-side sparse-hart, split-memory and clock-permission DTB fixture generation
also lives here. Kernel tests select scenarios and assert Kernel behavior without
carrying a firmware parser.
