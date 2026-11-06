mod case;
mod expectation;
mod state;
mod task;
pub use case::Case;

pub use expectation::Expectation;
pub use state::State;
pub use task::Task;

use crate::Error;
use unit_contract::event::{self, Event};

/// Diagnostic prose is ignored. Every protocol frame must be complete and valid.
pub fn events(serial: &[u8]) -> Result<Vec<Event<String>>, Error> {
    let mut result = Vec::new();
    let mut cursor = 0;
    while let Some(index) = serial[cursor..].iter().position(|byte| *byte == 0x1e) {
        let start = cursor + index;
        let end = serial[start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| start + offset + 1)
            .ok_or_else(|| Error::Workflow("truncated System event frame".into()))?;
        let frame = &serial[start..end];
        // UART firmware may expand LF to CRLF. Normalize that transport suffix
        // only; the typed codec still rejects malformed payloads and extra CRs.
        let canonical;
        let frame = if let Some(body) = frame.strip_suffix(b"\r\n") {
            canonical = [body, b"\n"].concat();
            canonical.as_slice()
        } else {
            frame
        };
        result.push(event::decode(frame).map_err(|e| Error::Workflow(e.to_string()))?);
        cursor = end;
    }
    Ok(result)
}

/// Judge native task exit receipts and acknowledged reclaim. No kernel event
/// logging is required, and diagnostic prose never supplies a passing verdict.
pub fn verify(serial: &[u8], unit: &str, expectation: Expectation) -> Result<(), Error> {
    let mut state = State::New;
    for event in events(serial)? {
        state = state.advance(event)?;
    }
    let State::Complete { tasks, .. } = state else {
        return Err(Error::Workflow("System completion event missing".into()));
    };
    let selected: Vec<_> = tasks.iter().filter(|(_, task)| task.unit == unit).collect();
    if selected.len() != 1 {
        return Err(Error::Workflow(format!(
            "expected one instance of test Unit {unit}, observed {}",
            selected.len()
        )));
    }
    let (_, task) = selected[0];
    let task::Phase::Retired(exit) = task.phase else {
        return Err(Error::Workflow(format!("test Unit {unit} was not retired")));
    };
    use unit_contract::event::Cause;
    let expected = match expectation {
        Expectation::Clean => exit.cause == Cause::Reap && exit.reason == 0,
        Expectation::Exit(reason) => exit.cause == Cause::Reap && exit.reason == reason,
        Expectation::Fault => exit.cause == Cause::Fault,
    };
    if !expected {
        return Err(Error::Workflow(format!(
            "unexpected native test exit: {exit:?}"
        )));
    }
    for task in tasks.values().filter(|task| task.unit != unit) {
        let task::Phase::Retired(exit) = task.phase else {
            unreachable!("completion validated every retirement");
        };
        if exit.cause == Cause::Fault || (exit.cause == Cause::Reap && exit.reason != 0) {
            return Err(Error::Workflow(format!(
                "unexpected native exit of Unit {}: {exit:?}",
                task.unit
            )));
        }
    }
    Ok(())
}

pub fn cases(root: &std::path::Path) -> Result<Vec<Case>, Error> {
    let project = crate::config::project(root)?;
    let files = crate::discovery::discover(root, &project)?;
    let mut cases = Vec::<Case>::new();
    for path in files.tests {
        let case: Case = crate::config::read(&path)?;
        if case.schema != 1
            || case.name.is_empty()
            || case.unit.is_empty()
            || case.timeout == Some(0)
        {
            return Err(Error::Declaration {
                path,
                message: "test needs schema=1, name, Unit ID and a nonzero timeout".into(),
            });
        }
        if cases.iter().any(|previous| previous.name == case.name) {
            return Err(Error::Declaration {
                path,
                message: format!("duplicate test name {}", case.name),
            });
        }
        cases.push(case);
    }
    cases.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(cases)
}

pub fn execute(root: &std::path::Path, case: &Case, offline: bool) -> Result<(), Error> {
    let project = crate::config::project(root)?;
    let mut runner = project
        .runner
        .ok_or_else(|| Error::Workflow("test requires [runner] in vanllia.toml".into()))?;
    if let Some(timeout) = case.timeout {
        runner.timeout = timeout;
    }
    let receipt = crate::build::build(
        root,
        &crate::Selection::Explicit(vec![case.unit.clone()]),
        &crate::build::Options { offline },
    )?;
    let result = crate::runner::run(root, &receipt, &runner)?;
    let verdict = verify(&result.serial, &case.unit, case.expected);
    let native_status = match case.expected {
        Expectation::Clean => result.status == 0,
        Expectation::Exit(_) | Expectation::Fault => result.status == 0 || result.status == 101,
    };
    let report = serde_json::json!({
        "schema": 1,
        "case": case,
        "passed": native_status && verdict.is_ok(),
        "vm_exit": result.status,
        "initrd_sha256": receipt.initrd.sha256,
        "kernel_sha256": receipt.kernel.as_ref().map(|kernel| &kernel.sha256),
        "firmware_sha256": receipt.firmware.as_ref().map(|firmware| &firmware.sha256),
        "error": verdict.as_ref().err().map(ToString::to_string),
    });
    vanllia_build::publish(
        &result.directory.join("verdict.json"),
        &serde_json::to_vec_pretty(&report).map_err(|e| Error::Workflow(e.to_string()))?,
    )
    .map_err(Error::Workflow)?;
    if !native_status || verdict.is_err() {
        return Err(Error::Workflow(format!(
            "{}: VM exit={}, verdict={verdict:?}; evidence: {}",
            case.name,
            result.status,
            result.directory.display()
        )));
    }
    Ok(())
}
