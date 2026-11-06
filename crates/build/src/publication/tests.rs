use super::*;

#[test]
fn releasing_a_lease_is_independent_of_inherited_child_descriptors() {
    let root = std::env::temp_dir().join(format!("vanllia-inherited-lock-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let output = root.join("output");
    let held = leases(&[&output]).unwrap();
    let mut pipe = [0; 2];
    assert_eq!(
        unsafe { libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC) },
        0
    );
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0);
    if pid == 0 {
        // Only async-signal-safe calls after fork in the threaded test process.
        unsafe {
            libc::close(pipe[1]);
            let mut byte = 0u8;
            libc::read(pipe[0], (&mut byte as *mut u8).cast(), 1);
            libc::_exit(0);
        }
    }
    unsafe {
        libc::close(pipe[0]);
    }
    drop(held);
    let next = leases(&[&output]);
    unsafe {
        let byte = 1u8;
        libc::write(pipe[1], (&byte as *const u8).cast(), 1);
        libc::close(pipe[1]);
        libc::waitpid(pid, std::ptr::null_mut(), 0);
    }
    assert!(
        next.is_ok(),
        "parent lease retained by child: {:?}",
        next.err()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn replacing_an_output_symlink_does_not_change_its_lock_identity() {
    let root = std::env::temp_dir().join(format!("vanllia-output-lock-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("old-image");
    let output = root.join("initrd.img");
    std::fs::write(&target, b"old").unwrap();
    std::os::unix::fs::symlink(&target, &output).unwrap();
    let held = leases(&[&output]).unwrap();
    let prepared = root.join("prepared");
    std::fs::write(&prepared, b"complete").unwrap();
    std::fs::rename(prepared, &output).unwrap();
    assert!(
        leases(&[&output]).is_err(),
        "a still-owned output acquired a second lock after rename"
    );
    assert_eq!(std::fs::read(target).unwrap(), b"old");
    drop(held);
    std::fs::remove_dir_all(root).unwrap();
}
