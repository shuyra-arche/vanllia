use elf::{ElfBytes, endian::LittleEndian, file::Class};
use env::ledger::capsule::PAGE;

use super::error::Error;
use super::region::Region;

pub(crate) struct Plan {
    pub(crate) entry: usize,
    pub(crate) regions: Vec<Region>,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Plan, Error> {
    let file = ElfBytes::<LittleEndian>::minimal_parse(bytes).map_err(|_| Error::Format)?;
    let header = &file.ehdr;
    if header.class != Class::ELF64
        || header.e_type != elf::abi::ET_EXEC
        || header.e_machine != elf::abi::EM_RISCV
        || header.version != 1
        || header.e_ehsize != 64
        || header.e_phentsize != 56
        || header.e_phnum == 0
    {
        return Err(Error::Unsupported);
    }
    let entry = usize::try_from(header.e_entry).map_err(|_| Error::Range)?;
    let segments = file.segments().ok_or(Error::Format)?;
    let count = usize::from(header.e_phnum);
    let mut regions: Vec<Region> = Vec::new();
    regions.try_reserve(count).map_err(|_| Error::Memory)?;
    let mut valid_entry = false;
    for i in 0..count {
        let segment = segments.get(i).map_err(|_| Error::Format)?;
        let kind = segment.p_type;
        if kind == elf::abi::PT_DYNAMIC || kind == elf::abi::PT_INTERP {
            return Err(Error::Unsupported);
        }
        if kind != elf::abi::PT_LOAD {
            continue;
        }
        let permissions = usize::try_from(segment.p_flags).map_err(|_| Error::Range)?;
        let offset = usize::try_from(segment.p_offset).map_err(|_| Error::Range)?;
        let address = usize::try_from(segment.p_vaddr).map_err(|_| Error::Range)?;
        let file_size = usize::try_from(segment.p_filesz).map_err(|_| Error::Range)?;
        let memory_size = usize::try_from(segment.p_memsz).map_err(|_| Error::Range)?;
        let alignment = usize::try_from(segment.p_align).map_err(|_| Error::Range)?;
        if memory_size < file_size
            || offset
                .checked_add(file_size)
                .is_none_or(|end| end > bytes.len())
        {
            return Err(Error::Range);
        }
        if alignment > 1
            && (!alignment.is_power_of_two() || address % alignment != offset % alignment)
        {
            return Err(Error::Format);
        }
        if permissions & !7 != 0 || permissions & 4 == 0 || permissions & 3 == 3 {
            return Err(Error::Permissions);
        }
        if memory_size == 0 {
            continue;
        }
        let end = address.checked_add(memory_size).ok_or(Error::Range)?;
        let flags =
            ((permissions & 4) >> 1 | (permissions & 2) << 1 | (permissions & 1) << 3) as u64;
        if flags & 8 != 0
            && entry >= address
            && entry
                .checked_add(2)
                .is_some_and(|end| end <= address + file_size)
        {
            valid_entry = true;
        }
        let va = address / PAGE * PAGE;
        let end = end.checked_next_multiple_of(PAGE).ok_or(Error::Range)?;
        if va == 0
            || regions
                .iter()
                .any(|region| va < region.va + region.size && region.va < end)
        {
            return Err(Error::Overlap);
        }
        let prefix = address - va;
        let size = end - va;
        let data_size = if flags & 8 != 0 {
            size
        } else if file_size == 0 {
            0
        } else {
            prefix
                .checked_add(file_size)
                .and_then(|n| n.checked_next_multiple_of(PAGE))
                .ok_or(Error::Range)?
        };
        regions.push(Region {
            va,
            size,
            data_size,
            flags,
            file_offset: offset,
            file_size,
            prefix,
        });
    }
    if !valid_entry || !entry.is_multiple_of(2) {
        return Err(Error::Entry);
    }
    Ok(Plan { entry, regions })
}
