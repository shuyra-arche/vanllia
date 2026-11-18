use std::{
    collections::BTreeSet,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use unit::{
    document::{Creation, Ending},
    event::Event,
};
use vanllia::testing::{Expectation, State};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Session {
    Startup,
    Password,
    Shell,
    Result,
    Return,
    Done,
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|part| part == needle)
}

/// Exercise the exact product artifact with the repository's public Login fixture.
pub fn run(
    root: &Path,
    artifact: &Path,
    firmware: &Path,
    hash: &str,
    length: u64,
    cpus: u8,
) -> Result<(), String> {
    if !(1..=4).contains(&cpus) {
        return Err("CPU count must be between 1 and 4".into());
    }
    let receipt = vanllia::runner::load(artifact).map_err(|e| e.to_string())?;
    let catalog = vanllia::load(root).map_err(|e| e.to_string())?;
    let plan = vanllia::plan(
        &catalog,
        &vanllia::Selection::Explicit(vec!["login".into()]),
    )
    .map_err(|e| e.to_string())?;
    let expected: BTreeSet<_> = plan
        .order
        .iter()
        .filter(|id| **id != plan.system)
        .filter(|id| catalog.units()[*id].unit.control.creation == Creation::Startup)
        .cloned()
        .collect();
    let endings: Vec<_> = expected
        .iter()
        .filter(|id| catalog.units()[*id].unit.control.ending == Ending::Told)
        .collect();
    let [ending] = endings.as_slice() else {
        return Err("expected one natural product ending".into());
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = root.join(".vanllia/runs").join(format!("login-{stamp}"));
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    use sha2::{Digest, Sha256};
    for (name, input) in [
        ("kernel.elf", receipt.kernel.as_ref().ok_or("kernel")?),
        ("initrd.img", &receipt.initrd),
    ] {
        let bytes =
            std::fs::read(receipt.directory.join(&input.path)).map_err(|e| e.to_string())?;
        if format!("{:x}", Sha256::digest(&bytes)) != input.sha256
            || bytes.len() as u64 != input.bytes
        {
            return Err("artifact changed before snapshot".into());
        }
        std::fs::write(directory.join(name), bytes).map_err(|e| e.to_string())?;
    }
    let bytes = std::fs::read(firmware).map_err(|e| e.to_string())?;
    let firmware_hash = format!("{:x}", Sha256::digest(&bytes));
    if firmware_hash != hash.to_ascii_lowercase() || bytes.len() as u64 != length || length == 0 {
        return Err("firmware identity".into());
    }
    std::fs::write(directory.join("firmware.bin"), bytes).map_err(|e| e.to_string())?;
    std::fs::write(directory.join("input.json"), serde_json::to_vec_pretty(&serde_json::json!({"artifact":receipt,"firmware_sha256":firmware_hash,"cpus":cpus,"expected_startup":expected,"input":"public Login fixture, arithmetic expression, Shell EOF, Login EOF"})).map_err(|e|e.to_string())?).map_err(|e| e.to_string())?;
    let mut command = Command::new("qemu-system-riscv64");
    command.current_dir(&directory).args([
        "-machine",
        "virt",
        "-m",
        "256M",
        "-smp",
        &cpus.to_string(),
        "-bios",
        "firmware.bin",
        "-kernel",
        "kernel.elf",
        "-initrd",
        "initrd.img",
        "-display",
        "none",
        "-chardev",
        "socket,id=console,path=console.sock,server=on,wait=on",
        "-serial",
        "chardev:console",
        "-monitor",
        "unix:monitor.sock,server=on,wait=off",
        "-no-reboot",
    ]);
    vanllia_build::workflow::process::initialize()?;
    let evidence = directory.join("process");
    let serial = directory.join("serial.log");
    let started = Instant::now();
    let mut sent_eof = false;
    let mut failure = None;
    let mut session = Session::Startup;
    let mut feedback_from = 0;
    let mut shell_task = None;
    let status = thread::scope(|scope| {
        let execution =
            scope.spawn(|| vanllia_build::workflow::process::observe(&mut command, &evidence, 120));
        let mut console = loop {
            if let Ok(stream) = UnixStream::connect(directory.join("console.sock")) {
                break Some(stream);
            }
            if execution.is_finished() || started.elapsed() > Duration::from_secs(10) {
                break None;
            }
            thread::sleep(Duration::from_millis(10));
        };
        if console.is_none() {
            failure = Some("console connection failed".into());
        }
        let capture = console
            .as_ref()
            .and_then(|stream| stream.try_clone().ok())
            .map(|mut stream| {
                let path = serial.clone();
                scope.spawn(move || -> Result<(), String> {
                    let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
                    let mut bytes = [0u8; 4096];
                    loop {
                        let n = stream.read(&mut bytes).map_err(|e| e.to_string())?;
                        if n == 0 {
                            return Ok(());
                        }
                        file.write_all(&bytes[..n]).map_err(|e| e.to_string())?;
                    }
                })
            });
        while !execution.is_finished()
            && failure.is_none()
            && started.elapsed() < Duration::from_secs(110)
        {
            if !sent_eof && let Ok(bytes) = std::fs::read(&serial) {
                // Judge complete frames only while the producer is active.
                let end = bytes
                    .iter()
                    .rposition(|b| *b == b'\n')
                    .map(|n| n + 1)
                    .unwrap_or(0);
                match vanllia::testing::events(&bytes[..end]) {
                    Err(e) => failure = Some(e.to_string()),
                    Ok(events) => {
                        let mut state = State::New;
                        let mut ready = BTreeSet::new();
                        let mut shell_retired = false;
                        for event in events {
                            if let Event::Retired { task } = &event
                                && Some(*task) == shell_task
                            {
                                shell_retired = true;
                            }
                            if let Event::Bound { unit, task, .. } = &event
                                && !expected.contains(unit)
                            {
                                if unit == "shell"
                                    && !matches!(session, Session::Startup | Session::Password)
                                {
                                    shell_task = Some(*task);
                                } else {
                                    failure =
                                        Some(format!("unexpected instance {unit} in {session:?}"));
                                    break;
                                }
                            }
                            if let Event::Ready { task } = &event
                                && let State::Running { tasks, .. } = &state
                                && let Some(row) = tasks.get(task)
                            {
                                ready.insert(row.unit.clone());
                            }
                            match state.advance(event) {
                                Ok(next) => state = next,
                                Err(e) => {
                                    failure = Some(e.to_string());
                                    break;
                                }
                            }
                        }
                        if failure.is_none() {
                            let tail = &bytes[feedback_from..];
                            let input: Option<&[u8]> = match session {
                                Session::Startup
                                    if ready == expected && contains(tail, b"login: ") =>
                                {
                                    session = Session::Password;
                                    Some(b"anran\n")
                                }
                                Session::Password if contains(tail, b"Password: ") => {
                                    session = Session::Shell;
                                    // The existing public Login host test supplies this fixture.
                                    Some(b"sqware\n")
                                }
                                Session::Shell
                                    if shell_task.is_some()
                                        && ready.contains("shell")
                                        && contains(tail, b"lisp> ") =>
                                {
                                    session = Session::Result;
                                    Some(b"(+ 314000 159)\n")
                                }
                                Session::Result
                                    if contains(tail, b"lisp> ")
                                        && String::from_utf8_lossy(tail).lines().any(|line| {
                                            line.trim().parse::<i64>() == Ok(314159)
                                        }) =>
                                {
                                    session = Session::Return;
                                    Some(&[4])
                                }
                                Session::Return if contains(tail, b"login: ") && shell_retired => {
                                    session = Session::Done;
                                    sent_eof = true;
                                    Some(&[4])
                                }
                                _ => None,
                            };
                            if let Some(input) = input
                                && let Some(stream) = console.as_mut()
                            {
                                feedback_from = bytes.len();
                                if let Err(error) = stream.write_all(input) {
                                    failure = Some(error.to_string());
                                }
                                println!("session: {session:?}");
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        if !execution.is_finished() {
            failure = Some(
                failure
                    .take()
                    .unwrap_or_else(|| "natural shutdown deadline".into()),
            );
            if let Ok(mut monitor) = UnixStream::connect(directory.join("monitor.sock")) {
                let _ = monitor.write_all(b"quit\n");
            }
        }
        let result = execution.join().map_err(|_| "observer panic".to_string())?;
        drop(console);
        if let Some(capture) = capture
            && let Err(error) = capture.join().map_err(|_| "capture panic".to_string())?
        {
            failure = Some(error);
        }
        result
    });
    let bytes = std::fs::read(&serial).map_err(|e| e.to_string())?;
    let verdict = vanllia::testing::verify(&bytes, ending, Expectation::Clean);
    let shell_verdict = vanllia::testing::verify(&bytes, "shell", Expectation::Clean);
    let passed = sent_eof
        && failure.is_none()
        && status.as_ref().is_ok_and(|s| s.success())
        && verdict.is_ok()
        && shell_verdict.is_ok();
    let report = serde_json::json!({"schema":1,"purpose":"login_requested_shell_command_cleanup","passed":passed,"sent_eof":sent_eof,
        "session":format!("{session:?}"),"shell_task":shell_task,"shell_verdict":format!("{shell_verdict:?}"),"command":"(+ 314000 159)","expected_result":314159,
        "error":failure,"native_verdict":format!("{verdict:?}"),"process":format!("{status:?}"),
        "host_forced_stop":failure.is_some(),"cpus":cpus,"initrd_sha256":receipt.initrd.sha256,
        "kernel_sha256":receipt.kernel.as_ref().unwrap().sha256,"firmware_sha256":firmware_hash});
    std::fs::write(
        directory.join("verdict.json"),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("evidence: {}\n{}", directory.display(), report);
    if passed {
        Ok(())
    } else {
        Err("login and requested Shell lifecycle failed".into())
    }
}
