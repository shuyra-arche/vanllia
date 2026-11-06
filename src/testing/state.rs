use super::{Task, task::Phase};
use crate::Error;
use std::collections::BTreeMap;
use unit_contract::event::Event;

/// State variants carry their valid observations; no success/finished flags.
#[derive(Debug)]
pub enum State {
    New,
    Running {
        system: u64,
        tasks: BTreeMap<u64, Task>,
    },
    Complete {
        system: u64,
        tasks: BTreeMap<u64, Task>,
    },
}
impl State {
    pub fn advance(self, event: Event<String>) -> Result<Self, Error> {
        let invalid = || Error::Workflow("invalid System event transition".into());
        match (self, event) {
            (Self::New, Event::Started { system }) if system != 0 => Ok(Self::Running {
                system,
                tasks: BTreeMap::new(),
            }),
            (Self::Running { system, mut tasks }, Event::Bound { unit, task, team })
                if task != 0
                    && task != system
                    && team != 0
                    && !unit.is_empty()
                    && !tasks.contains_key(&task) =>
            {
                tasks.insert(
                    task,
                    Task {
                        unit,
                        team,
                        phase: Phase::Created,
                    },
                );
                Ok(Self::Running { system, tasks })
            }
            (Self::Running { system, mut tasks }, Event::Ready { task }) => {
                let task = tasks.get_mut(&task).ok_or_else(invalid)?;
                if task.phase != Phase::Created {
                    return Err(invalid());
                }
                task.phase = Phase::Ready;
                Ok(Self::Running { system, tasks })
            }
            (Self::Running { system, mut tasks }, Event::Exited { task, exit }) => {
                let task = tasks.get_mut(&task).ok_or_else(invalid)?;
                if !matches!(task.phase, Phase::Created | Phase::Ready) {
                    return Err(invalid());
                }
                task.phase = Phase::Exited(exit);
                Ok(Self::Running { system, tasks })
            }
            (Self::Running { system, mut tasks }, Event::Retired { task }) => {
                let task = tasks.get_mut(&task).ok_or_else(invalid)?;
                let Phase::Exited(exit) = task.phase else {
                    return Err(invalid());
                };
                task.phase = Phase::Retired(exit);
                Ok(Self::Running { system, tasks })
            }
            (
                Self::Running { system, tasks },
                Event::Complete {
                    system: finished,
                    reason: 0,
                },
            ) if system == finished => {
                if tasks
                    .values()
                    .any(|task| !matches!(task.phase, Phase::Retired(_)))
                {
                    return Err(Error::Workflow(
                        "System completed with unreclaimed Unit instances".into(),
                    ));
                }
                Ok(Self::Complete { system, tasks })
            }
            _ => Err(invalid()),
        }
    }
}
