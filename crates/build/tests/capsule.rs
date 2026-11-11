use kernel_api::{
    abi,
    boot::capsule::{Capsule, PAGE, VERSION},
};
use vanllia_build::{boot_capsule, validate_boot_capsule};

fn elf() -> Vec<u8> {
    let mut bytes = vec![0; PAGE + 4];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x10000u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
    bytes[64..68].copy_from_slice(&1u32.to_le_bytes());
    bytes[68..72].copy_from_slice(&5u32.to_le_bytes());
    bytes[72..80].copy_from_slice(&(PAGE as u64).to_le_bytes());
    bytes[80..88].copy_from_slice(&0x10000u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&4u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&(PAGE as u64).to_le_bytes());
    bytes[112..120].copy_from_slice(&(PAGE as u64).to_le_bytes());
    bytes[PAGE..PAGE + 4].copy_from_slice(&[0x13, 0, 0, 0]);
    bytes
}

#[test]
fn generated_capsule_is_current_kernel_input_with_page_complete_executable_payload() {
    let bytes = boot_capsule(&elf()).unwrap();
    assert_eq!(&bytes[8..10], &VERSION.to_le_bytes());
    assert_eq!(bytes[11], abi::VERSION as u8);
    let capsule = Capsule::parse(&bytes).unwrap();
    assert_eq!(capsule.entry, 0x10000);
    assert_eq!(capsule.count, 1);
    let region = capsule.region(0).unwrap();
    assert_eq!(
        (region.va, region.pages, region.data_pages, region.flags),
        (0x10000, 1, 1, 10)
    );
    assert_eq!(&bytes[region.payload..region.payload + 4], &[0x13, 0, 0, 0]);
    assert!(
        bytes[region.payload + 4..region.payload + PAGE]
            .iter()
            .all(|byte| *byte == 0)
    );
    validate_boot_capsule(&bytes).unwrap();
    for (at, value) in [(8, 1), (11, 8), (10, 13), (16, 1), (64, 7)] {
        let mut stale = bytes.clone();
        stale[at] = value;
        assert!(
            validate_boot_capsule(&stale).is_err(),
            "accepted changed byte {at}"
        );
    }
    for end in [0, 8, 31, PAGE, bytes.len() - 1] {
        assert!(validate_boot_capsule(&bytes[..end]).is_err());
    }
}

#[test]
fn wrong_architecture_dynamic_images_and_non_file_entry_points_fail_before_packaging() {
    for (at, replacement) in [
        (4, vec![1]),                                   // ELF32
        (5, vec![2]),                                   // big endian
        (16, 3u16.to_le_bytes().to_vec()),              // ET_DYN
        (18, 62u16.to_le_bytes().to_vec()),             // x86-64
        (24, 0x10001u64.to_le_bytes().to_vec()),        // odd entry
        (24, 0x10004u64.to_le_bytes().to_vec()),        // executable BSS is not an entry
        (68, 7u32.to_le_bytes().to_vec()),              // W+X
        (96, (PAGE as u64 + 8).to_le_bytes().to_vec()), // truncated payload
    ] {
        let mut bytes = elf();
        bytes[at..at + replacement.len()].copy_from_slice(&replacement);
        assert!(
            boot_capsule(&bytes).is_err(),
            "accepted changed ELF field {at}"
        );
    }
}
