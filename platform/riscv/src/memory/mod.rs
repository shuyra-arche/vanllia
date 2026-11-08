//! Decode firmware before allocation, then admit normalized facts through Kernel API.
mod ranges;
mod tree;
use crate::Region;
use fdt::node::FdtNode;
use kernel_api::address::PAGE_SIZE;
pub use kernel_api::boot::platform::{Memory, MemoryError};
use ranges::{Ranges, checked_end, covering_pages};
const MAX_RAM: usize = 16;
const MAX_RESERVED: usize = 64;
pub fn parse(bytes: &[u8], dtp: usize, kernel: Region) -> Result<Memory, MemoryError> {
    let reservations = tree::validate(bytes)?;
    let fdt = fdt::Fdt::new(bytes).map_err(|_| MemoryError::Description)?;
    let root = fdt.find_node("/").ok_or(MemoryError::Description)?;
    let cells = (
        cells(root, "#address-cells", 2)?,
        cells(root, "#size-cells", 1)?,
    );
    for name in ["reserved-memory", "chosen"] {
        if root
            .children()
            .filter(|node| node.name.split('@').next() == Some(name))
            .count()
            > 1
        {
            return Err(MemoryError::Description);
        }
    }
    let mut ram = Ranges::<MAX_RAM>::new();
    for node in root.children() {
        if node.name.split('@').next() != Some("memory") {
            continue;
        }
        if !enabled(node)? {
            continue;
        }
        if property(node, "device_type")?.is_some_and(|p| p != b"memory\0") {
            return Err(MemoryError::Description);
        }
        regions(node, cells, |r| {
            if !r.base.is_multiple_of(PAGE_SIZE) || !r.size.is_multiple_of(PAGE_SIZE) {
                return Err(MemoryError::Range);
            }
            ram.insert(r, false)
        })?;
    }
    let dtb = Region::new(dtp, bytes.len());
    let initrd = initrd(&fdt)?;
    let mut reserved: Ranges<MAX_RESERVED> = Ranges::new();
    for entry in bytes[reservations].chunks_exact(16) {
        if entry == [0; 16] {
            break;
        }
        reserved.insert(tree::reservation(entry)?, true)?;
    }
    let mut no_map: Ranges<MAX_RESERVED> = Ranges::new();
    if let Some(parent) = fdt.find_node("/reserved-memory") {
        if !enabled(parent)? {
            return Err(MemoryError::Description);
        }
        if (
            self::cells(parent, "#address-cells", 0)?,
            self::cells(parent, "#size-cells", 0)?,
        ) != cells
            || property(parent, "ranges")? != Some(&[][..])
        {
            return Err(MemoryError::Description);
        }
        for node in parent.children() {
            if enabled(node)? {
                // Dynamic `size` reservations cannot be resolved before boot;
                // do not silently treat their unknown storage as free memory.
                let excluded = property(node, "no-map")?;
                if excluded.is_some_and(|p| !p.is_empty()) {
                    return Err(MemoryError::Description);
                }
                regions(node, cells, |r| {
                    reserved.insert(r, true)?;
                    if excluded.is_some() {
                        no_map.insert(covering_pages(r)?, true)?;
                    }
                    Ok(())
                })?;
            }
        }
    }
    Memory::new(
        ram.as_slice(),
        reserved.as_slice(),
        no_map.as_slice(),
        dtb,
        kernel,
        initrd,
    )
}
fn property<'a>(node: FdtNode<'_, 'a>, name: &str) -> Result<Option<&'a [u8]>, MemoryError> {
    let mut properties = node.properties().filter(|p| p.name == name);
    let value = properties.next().map(|p| p.value);
    if properties.next().is_some() {
        return Err(MemoryError::Description);
    }
    Ok(value)
}

fn enabled(node: FdtNode<'_, '_>) -> Result<bool, MemoryError> {
    match property(node, "status")? {
        None | Some(b"ok\0" | b"okay\0") => Ok(true),
        Some(b"disabled\0" | b"reserved\0" | b"fail\0") => Ok(false),
        Some(s) if s.starts_with(b"fail-") && s.ends_with(&[0]) => Ok(false),
        _ => Err(MemoryError::Description),
    }
}

fn cells(node: FdtNode<'_, '_>, name: &str, default: usize) -> Result<usize, MemoryError> {
    let n = match property(node, name)? {
        None => default,
        Some(p) if p.len() == 4 => u32::from_be_bytes(p.try_into().unwrap()) as usize,
        _ => return Err(MemoryError::Description),
    };
    if matches!(n, 1 | 2) {
        Ok(n)
    } else {
        Err(MemoryError::Description)
    }
}

fn number(bytes: &[u8]) -> Result<usize, MemoryError> {
    let n = match bytes.len() {
        4 => u32::from_be_bytes(bytes.try_into().unwrap()) as u64,
        8 => u64::from_be_bytes(bytes.try_into().unwrap()),
        _ => return Err(MemoryError::Description),
    };
    usize::try_from(n).map_err(|_| MemoryError::Range)
}

fn regions(
    node: FdtNode<'_, '_>,
    (address, size): (usize, usize),
    mut take: impl FnMut(Region) -> Result<(), MemoryError>,
) -> Result<(), MemoryError> {
    let bytes = property(node, "reg")?.ok_or(MemoryError::Description)?;
    let stride = (address + size) * 4;
    if bytes.is_empty() || !bytes.len().is_multiple_of(stride) {
        return Err(MemoryError::Description);
    }
    for entry in bytes.chunks_exact(stride) {
        let r = Region::new(
            number(&entry[..address * 4])?,
            number(&entry[address * 4..])?,
        );
        checked_end(r)?;
        take(r)?;
    }
    Ok(())
}

fn initrd(fdt: &fdt::Fdt<'_>) -> Result<Option<Region>, MemoryError> {
    let Some(chosen) = fdt.find_node("/chosen") else {
        return Ok(None);
    };
    let start = property(chosen, "linux,initrd-start")?;
    let end = property(chosen, "linux,initrd-end")?;
    if start.is_none() && end.is_none() {
        return Ok(None);
    }
    let start = number(start.ok_or(MemoryError::Description)?)?;
    let end = number(end.ok_or(MemoryError::Description)?)?;
    if start == 0 || !start.is_multiple_of(PAGE_SIZE) || end <= start {
        return Err(MemoryError::Range);
    }
    Ok(Some(covering_pages(Region::new(start, end - start))?))
}
