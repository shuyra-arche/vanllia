use super::{MemoryError, Region};
use kernel_api::address::PAGE_SIZE;

/// Bounded, sorted, disjoint byte ranges. All arithmetic is checked at admission.
/// Capacity is an explicit early-boot limit, never permission to drop a range.
#[derive(Clone, Copy, Debug)]
pub struct Ranges<const N: usize> {
    entries: [Region; N],
    len: usize,
}

impl<const N: usize> Ranges<N> {
    pub const fn new() -> Self {
        Self {
            entries: [Region::new(0, 0); N],
            len: 0,
        }
    }

    pub fn as_slice(&self) -> &[Region] {
        &self.entries[..self.len]
    }

    pub fn insert(&mut self, region: Region, overlap: bool) -> Result<(), MemoryError> {
        let mut start = region.base;
        let mut end = checked_end(region)?;
        let mut first = 0;
        while first < self.len && self.entries[first].range().end < start {
            first += 1;
        }
        let mut last = first;
        while last < self.len && self.entries[last].base <= end {
            let old = self.entries[last];
            if !overlap && start < old.range().end && old.base < end {
                return Err(MemoryError::Overlap);
            }
            start = start.min(old.base);
            end = end.max(old.range().end);
            last += 1;
        }
        if first == last {
            if self.len == N {
                return Err(MemoryError::Capacity);
            }
            self.entries.copy_within(first..self.len, first + 1);
            self.len += 1;
        } else {
            self.entries.copy_within(last..self.len, first + 1);
            self.len -= last - first - 1;
        }
        self.entries[first] = Region::new(start, end - start);
        Ok(())
    }
}

pub fn checked_end(region: Region) -> Result<usize, MemoryError> {
    if region.size == 0 {
        return Err(MemoryError::Range);
    }
    region
        .base
        .checked_add(region.size)
        .ok_or(MemoryError::Range)
}

pub fn covering_pages(region: Region) -> Result<Region, MemoryError> {
    let end = checked_end(region)?
        .checked_next_multiple_of(PAGE_SIZE)
        .ok_or(MemoryError::Range)?;
    let start = region.base / PAGE_SIZE * PAGE_SIZE;
    Ok(Region::new(start, end - start))
}
