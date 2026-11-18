//! Test-only initrd envelope for the production Vanllia boot Capsule encoder.
//! ABI-negative cases change only the byte under test, not ELF normalization.
use kernel_verification::{Result, require};

pub fn initrd(elf: &[u8], version: u8) -> Result<Vec<u8>> {
    require(matches!(version, 10..=13), "unsupported test ABI version")?;
    let mut capsule = vanllia_build::boot_capsule(elf)?;
    require(
        capsule.get(11) == Some(&13),
        "test expects the pinned product ABI 13 capsule",
    )?;
    capsule[11] = version;
    let mut initrd = vec![0; 4096];
    initrd[..4].copy_from_slice(&4096u32.to_le_bytes());
    initrd[4..8].copy_from_slice(&u32::try_from(capsule.len())?.to_le_bytes());
    initrd.extend_from_slice(&capsule);
    Ok(initrd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_abi_without_parsing_an_elf() {
        assert!(initrd(&[], 9).is_err());
        assert!(initrd(&[], 14).is_err());
    }
}
