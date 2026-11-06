mod error;
mod plan;
mod region;

use env::ledger::capsule::{HEADER, MAGIC, PAGE, RECORD};

use error::Error;

pub(crate) fn encode(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let plan = plan::parse(bytes)?;
    let table = HEADER
        .checked_add(plan.regions.len().checked_mul(RECORD).ok_or(Error::Range)?)
        .ok_or(Error::Range)?;
    let mut length = table.checked_next_multiple_of(PAGE).ok_or(Error::Range)?;
    for region in &plan.regions {
        length = length.checked_add(region.data_size).ok_or(Error::Range)?;
    }
    let mut out = Vec::new();
    out.try_reserve_exact(length).map_err(|_| Error::Memory)?;
    out.resize(length, 0);
    out[..8].copy_from_slice(&MAGIC);
    out[8..10].copy_from_slice(&1u16.to_le_bytes());
    out[10] = 12;
    out[12..16].copy_from_slice(
        &u32::try_from(plan.regions.len())
            .map_err(|_| Error::Range)?
            .to_le_bytes(),
    );
    out[16..24].copy_from_slice(&(plan.entry as u64).to_le_bytes());
    out[24..32].copy_from_slice(&(length as u64).to_le_bytes());
    let mut payload = table.next_multiple_of(PAGE);
    for (i, region) in plan.regions.iter().enumerate() {
        let at = HEADER + i * RECORD;
        let offset = if region.data_size == 0 { 0 } else { payload };
        for (field, value) in [
            region.va,
            region.size / PAGE,
            region.data_size / PAGE,
            offset,
        ]
        .into_iter()
        .enumerate()
        {
            out[at + field * 8..at + field * 8 + 8].copy_from_slice(&(value as u64).to_le_bytes());
        }
        let permissions = (region.flags >> 1) as u32;
        out[at + 32..at + 36].copy_from_slice(&permissions.to_le_bytes());
        if region.file_size > 0 {
            out[payload + region.prefix..payload + region.prefix + region.file_size]
                .copy_from_slice(&bytes[region.file_offset..region.file_offset + region.file_size]);
        }
        payload += region.data_size;
    }
    Ok(out)
}
