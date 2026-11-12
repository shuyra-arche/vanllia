//! The first provider supports exclusive level-high QEMU MMIO devices directly
//! wired to the selected PLIC. Other gateways require their own explicit contract.
use crate::tree::{self, Error, property, scalar};
use alloc::vec::Vec;
pub(super) fn discover(
    fdt: &fdt::Fdt,
    parent: u32,
    limit: usize,
    reserved: &[u32],
) -> Result<Vec<(u32, usize)>, Error> {
    let mut sources = Vec::new();
    for node in fdt.all_nodes() {
        if !tree::enabled(fdt, node)? {
            continue;
        }
        let compatible = |wanted| {
            node.compatible()
                .is_some_and(|c| c.all().any(|v| v == wanted))
        };
        if property(node, "interrupt-controller")?.is_some()
            || compatible("riscv,plic0")
            || compatible("sifive,plic-1.0.0")
            || compatible("riscv,iommu")
            || compatible("riscv,clint0")
            || compatible("sifive,clint0")
            || compatible("riscv,aclint-mtimer")
            || compatible("riscv,aclint-mswi")
            || compatible("riscv,aclint-sswi")
        {
            continue;
        }
        let direct = property(node, "interrupts")?;
        let extended = property(node, "interrupts-extended")?;
        if direct.is_none() && extended.is_none() {
            continue;
        }
        // This is a hardware-provider guarantee, never inferred from an IRQ ID.
        if !(compatible("ns16550a")
            || compatible("virtio,mmio")
            || compatible("google,goldfish-rtc"))
            || direct.is_some() && extended.is_some()
            || property(node, "msi-parent")?.is_some()
            || property(node, "interrupt-map")?.is_some()
        {
            return Err(Error::Description);
        }
        let mut ancestor = Some(node);
        while let Some(current) = ancestor {
            if property(current, "interrupt-map")?.is_some()
                || property(current, "msi-parent")?.is_some()
            {
                return Err(Error::Description);
            }
            ancestor = tree::parent(fdt, current);
        }
        let mut lines = Vec::new();
        if let Some(bytes) = extended {
            if bytes.is_empty() || !bytes.len().is_multiple_of(8) {
                return Err(Error::Description);
            }
            for pair in bytes.chunks_exact(8) {
                if tree::word(&pair[..4])? != parent {
                    return Err(Error::Description);
                }
                lines.try_reserve(1).map_err(|_| Error::OutOfMemory)?;
                lines.push(tree::word(&pair[4..])?);
            }
        } else {
            let mut ancestor = node;
            loop {
                if property(ancestor, "interrupt-map")?.is_some() {
                    return Err(Error::Description);
                }
                if let Some(id) = scalar(ancestor, "interrupt-parent")? {
                    if id != parent {
                        return Err(Error::Description);
                    }
                    break;
                }
                ancestor = tree::parent(fdt, ancestor).ok_or(Error::Description)?;
            }
            let bytes = direct.ok_or(Error::Description)?;
            if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
                return Err(Error::Description);
            }
            for word in bytes.chunks_exact(4) {
                lines.try_reserve(1).map_err(|_| Error::OutOfMemory)?;
                lines.push(tree::word(word)?);
            }
        }
        let region = tree::mmio::single(fdt, node)?;
        for line in lines {
            if line == 0
                || line as usize > limit
                || reserved.contains(&line)
                || sources.iter().any(|(prior, _)| *prior == line)
                || sources.len() == 64
            {
                return Err(Error::Description);
            }
            sources.try_reserve(1).map_err(|_| Error::OutOfMemory)?;
            sources.push((line, region.base));
        }
    }
    Ok(sources)
}
