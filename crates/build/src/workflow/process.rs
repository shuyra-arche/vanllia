use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::publication;

pub fn initialize() -> Result<(), String> {
    // Owned orphaned descendants must remain waitable by this process.
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) } != 0 {
        return Err(format!(
            "cannot establish child reaping: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

struct Execution {
    child: std::process::Child,
    owns_group: bool,
    waited: bool,
}

impl Drop for Execution {
    fn drop(&mut self) {
        if !self.waited {
            let pid = self.child.id();
            let stop = stop(pid, self.owns_group);
            let wait = self.child.wait();
            if stop.is_err() || wait.is_err() {
                eprintln!("execution cleanup failed: stop={stop:?}, wait={wait:?}");
            }
        }
    }
}

fn reap_group(pid: u32) {
    // Only children adopted by this process are eligible; other groups are untouched.
    while unsafe { libc::waitpid(-(pid as i32), std::ptr::null_mut(), libc::WNOHANG) } > 0 {}
}

fn group_present(pid: u32) -> Result<bool, String> {
    if unsafe { libc::kill(-(pid as i32), 0) } == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(false)
    } else {
        Err(error.to_string())
    }
}

fn stop(pid: u32, group: bool) -> Result<(), String> {
    let target = if group { -(pid as i32) } else { pid as i32 };
    if unsafe { libc::kill(target, libc::SIGKILL) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(error.to_string())
    }
}

fn drain_group(pid: u32) -> Result<bool, String> {
    // An existing driver may signal its helpers just before returning. Reap and
    // wait for those stops to complete; a live group after this bound still fails.
    for _ in 0..20 {
        reap_group(pid);
        if !group_present(pid)? {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}

pub fn capture(command: &mut Command) -> Result<Vec<u8>, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let evidence = std::env::temp_dir().join(format!(
        "vanllia-capture-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    run(command, &evidence, 600).map_err(|e| {
        format!(
            "{e}; evidence: {}\n{}",
            evidence.display(),
            std::fs::read_to_string(evidence.join("stderr.log")).unwrap_or_default()
        )
    })?;
    let output = std::fs::read(evidence.join("stdout.log")).map_err(|e| e.to_string())?;
    std::fs::remove_dir_all(&evidence)
        .map_err(|e| format!("cleanup {}: {e}", evidence.display()))?;
    Ok(output)
}

pub fn run(command: &mut Command, evidence: &Path, timeout: u64) -> Result<(), String> {
    let status = observe(command, evidence, timeout)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("execution failed: {status}"))
    }
}

/// Observe a native exit without converting an expected nonzero test exit into
/// a transport failure. Cancellation, timeout and cleanup still return errors.
pub fn observe(
    command: &mut Command,
    evidence: &Path,
    timeout: u64,
) -> Result<std::process::ExitStatus, String> {
    super::active()?;
    if timeout == 0 {
        return Err("timeout must be nonzero".into());
    }
    std::fs::create_dir_all(evidence).map_err(|e| e.to_string())?;
    let stdout = File::create(evidence.join("stdout.log")).map_err(|e| e.to_string())?;
    let stderr = File::create(evidence.join("stderr.log")).map_err(|e| e.to_string())?;
    command.stdin(Stdio::null()).stdout(stdout).stderr(stderr);
    // Each execution owns a process group. Cancellation of this CLI and timeout
    // terminate that entire group, then wait for the owned child before returning.
    use std::os::unix::process::CommandExt;
    let owns_group = std::env::var_os("VANLLIA_MANAGED_GROUP").is_none();
    if owns_group {
        command.process_group(0).env("VANLLIA_MANAGED_GROUP", "1");
    }
    let child = command.spawn().map_err(|e| e.to_string())?;
    let pid = child.id();
    let mut execution = Execution {
        child,
        owns_group,
        waited: false,
    };
    let started = Instant::now();
    let mut exit_status = None;
    let result = (|| {
        publication::write(
            &evidence.join("execution.json"),
            &serde_json::to_vec_pretty(&serde_json::json!({
                "pid": pid, "command": format!("{command:?}"), "timeout_seconds": timeout,
            }))
            .unwrap(),
        )?;
        loop {
            if let Some(status) = execution.child.try_wait().map_err(|e| e.to_string())? {
                execution.waited = true;
                exit_status = Some(status);
                break Ok(());
            }
            if started.elapsed() >= Duration::from_secs(timeout) || super::cancelled() {
                let stop = stop(pid, owns_group);
                let wait = execution.child.wait();
                execution.waited = wait.is_ok();
                break Err(format!(
                    "execution {}: stop={stop:?}, wait={wait:?}",
                    if super::cancelled() {
                        "cancelled"
                    } else {
                        "timed out"
                    }
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    })();
    // Evidence I/O and wait errors must take the same stop path as a timeout.
    if !execution.waited {
        let stopped = stop(pid, owns_group);
        let waited = execution.child.wait();
        execution.waited = waited.is_ok();
        if stopped.is_err() || waited.is_err() {
            eprintln!("execution stop failed: {stopped:?}, wait={waited:?}");
        }
    }
    // A successful leader may not leave a background driver/QEMU behind.
    let stopped = !owns_group || drain_group(pid).unwrap_or(false);
    let result = if !stopped {
        let diagnostics = Command::new("ps")
            .args(["-eo", "pid,ppid,pgid,state,comm"])
            .output()
            .map_err(|e| e.to_string())
            .and_then(|snapshot| {
                publication::write(&evidence.join("cleanup-processes.log"), &snapshot.stdout)
            });
        let cleanup = stop(pid, true);
        let confirmed = drain_group(pid).unwrap_or(false);
        if !confirmed {
            let marker = publication::write(
                &evidence.join("STOP_UNCONFIRMED"),
                format!("process group {pid}").as_bytes(),
            );
            if let Err(error) = marker {
                eprintln!("reservation evidence failed: {error}");
            }
        }
        Err(format!(
            "runner left active descendants; cleanup={cleanup:?}, stop_confirmed={confirmed}; diagnostics={diagnostics:?}; prior={result:?}"
        ))
    } else {
        result
    };
    record_result(evidence, pid, started.elapsed().as_millis(), result)?;
    let status = exit_status.ok_or("execution exited without a native status")?;
    use std::os::unix::process::ExitStatusExt;
    publication::write(
        &evidence.join("exit.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "code": status.code(), "signal": status.signal(), "success": status.success(),
        }))
        .map_err(|e| e.to_string())?,
    )?;
    Ok(status)
}

fn record_result(
    evidence: &Path,
    pid: u32,
    elapsed_ms: u128,
    result: Result<(), String>,
) -> Result<(), String> {
    publication::write(&evidence.join("result.json"), &serde_json::to_vec_pretty(&serde_json::json!({
        "pid": pid, "success": result.is_ok(), "result": format!("{result:?}"), "elapsed_ms": elapsed_ms,
    })).unwrap()).map_err(|record| format!("execution={result:?}; result record failed: {record}"))?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_preserves_native_failure_without_losing_evidence() {
        let path = std::env::temp_dir().join(format!("vanllia-native-exit-{}", std::process::id()));
        let status = observe(Command::new("sh").args(["-c", "exit 17"]), &path, 5).unwrap();
        assert_eq!(status.code(), Some(17));
        let exit: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("exit.json")).unwrap()).unwrap();
        assert_eq!(exit["code"], 17);
        assert_eq!(exit["success"], false);
        assert!(run(Command::new("sh").args(["-c", "exit 17"]), &path, 5).is_err());
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn timeout_stops_owned_execution_and_keeps_evidence() {
        let path =
            std::env::temp_dir().join(format!("vanllia-timeout-test-{}", std::process::id()));
        let error = run(Command::new("sleep").arg("30"), &path, 1).unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        let execution: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("execution.json")).unwrap()).unwrap();
        let pid = execution["pid"].as_u64().unwrap();
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
        assert!(path.join("result.json").is_file());
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn successful_leader_cannot_leave_a_live_descendant() {
        assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) }, 0);
        let path =
            std::env::temp_dir().join(format!("vanllia-descendant-test-{}", std::process::id()));
        let error = run(
            Command::new("sh").args(["-c", "sleep 30 & echo $!"]),
            &path,
            5,
        )
        .unwrap_err();
        assert!(error.contains("runner left active descendants"), "{error}");
        assert!(error.contains("stop_confirmed=true"), "{error}");
        let pid = std::fs::read_to_string(path.join("stdout.log")).unwrap();
        assert!(!Path::new(&format!("/proc/{}", pid.trim())).exists());
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn evidence_write_failure_still_stops_owned_child() {
        let path =
            std::env::temp_dir().join(format!("vanllia-evidence-failure-{}", std::process::id()));
        std::fs::create_dir_all(path.join("execution.json")).unwrap();
        assert!(run(Command::new("sleep").arg("30"), &path, 5).is_err());
        let result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("result.json")).unwrap()).unwrap();
        assert_eq!(result["success"], false);
        let pid = result["pid"].as_u64().unwrap();
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn result_record_failure_preserves_unconfirmed_stop() {
        let path =
            std::env::temp_dir().join(format!("vanllia-result-failure-{}", std::process::id()));
        std::fs::create_dir_all(path.join("result.json")).unwrap();
        let error = record_result(&path, 0, 0, Err("stop_confirmed=false".into())).unwrap_err();
        assert!(error.contains("stop_confirmed=false"), "{error}");
        assert!(error.contains("result record failed"), "{error}");
        std::fs::remove_dir_all(path).unwrap();
    }
}
