use unit_contract::event::{self, Cause, Event, Exit};
use vanllia::testing::{self, Expectation, State};

fn serial(exit: Exit) -> Vec<u8> {
    let mut bytes = b"ordinary diagnostic prose\n".to_vec();
    for event in [
        Event::Started { system: 1 },
        Event::Bound {
            unit: "test",
            task: 9,
            team: 3,
        },
        Event::Ready { task: 9 },
        Event::Exited { task: 9, exit },
        Event::Retired { task: 9 },
        Event::Complete {
            system: 1,
            reason: 0,
        },
    ] {
        bytes.extend(event::encode(&event).unwrap().as_bytes());
    }
    bytes
}
fn clean() -> Exit {
    Exit {
        cause: Cause::Reap,
        reason: 0,
    }
}

#[test]
fn native_exit_and_reclaim_pass_without_any_kernel_log() {
    testing::verify(&serial(clean()), "test", Expectation::Clean).unwrap();
}

#[test]
fn uart_crlf_preserves_typed_receipts_without_accepting_extra_control_bytes() {
    let bytes = serial(clean());
    let crlf: Vec<_> = bytes
        .iter()
        .flat_map(|byte| {
            if *byte == b'\n' {
                vec![b'\r', b'\n']
            } else {
                vec![*byte]
            }
        })
        .collect();
    testing::verify(&crlf, "test", Expectation::Clean).unwrap();
    let malformed: Vec<_> = crlf
        .iter()
        .flat_map(|byte| {
            if *byte == b'\r' {
                vec![b'\r', b'\r']
            } else {
                vec![*byte]
            }
        })
        .collect();
    assert!(testing::verify(&malformed, "test", Expectation::Clean).is_err());
}

#[test]
fn missing_completion_and_every_truncated_frame_fail() {
    let bytes = serial(clean());
    let final_frame = event::encode(&Event::Complete {
        system: 1,
        reason: 0,
    })
    .unwrap();
    let start = bytes.len() - final_frame.len();
    for end in start..bytes.len() {
        assert!(testing::verify(&bytes[..end], "test", Expectation::Clean).is_err());
    }
    assert_eq!(testing::events(&bytes).unwrap().len(), 6);
}

#[test]
fn success_text_and_forced_termination_cannot_substitute_for_native_success() {
    assert!(testing::verify(b"PASS\n", "test", Expectation::Clean).is_err());
    assert!(testing::verify(&serial(clean()), "missing", Expectation::Clean).is_err());
    for cause in [Cause::Slay, Cause::Cascade, Cause::Fault] {
        assert!(
            testing::verify(
                &serial(Exit { cause, reason: 0 }),
                "test",
                Expectation::Clean
            )
            .is_err()
        );
    }
}

#[test]
fn expected_fault_is_a_native_exit_category_and_not_a_success_string() {
    let fault = Exit {
        cause: Cause::Fault,
        reason: env::EXIT_FAULT as u64,
    };
    testing::verify(&serial(fault), "test", Expectation::Fault).unwrap();
    assert!(testing::verify(&serial(fault), "test", Expectation::Clean).is_err());
    assert!(testing::verify(&serial(clean()), "test", Expectation::Fault).is_err());
    let code = Exit {
        cause: Cause::Reap,
        reason: 17,
    };
    testing::verify(&serial(code), "test", Expectation::Exit(17)).unwrap();
    assert!(testing::verify(&serial(code), "test", Expectation::Exit(18)).is_err());
}

#[test]
fn invalid_state_transitions_and_reclaim_without_an_exit_receipt_fail() {
    assert!(State::New.advance(Event::Ready { task: 9 }).is_err());
    let state = State::New
        .advance(Event::Started { system: 1 })
        .unwrap()
        .advance(Event::Bound {
            unit: "test".into(),
            task: 9,
            team: 3,
        })
        .unwrap();
    assert!(state.advance(Event::Retired { task: 9 }).is_err());
    let state = State::New.advance(Event::Started { system: 1 }).unwrap();
    assert!(
        state
            .advance(Event::Complete {
                system: 2,
                reason: 0
            })
            .is_err()
    );
}
