use crate::operation::Operation;
use std::path::PathBuf;
use vanllia::Selection;

pub(crate) struct Invocation {
    pub operation: Operation,
    pub root: PathBuf,
    pub selection: Selection,
}

impl Invocation {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
        let mut args = args.into_iter();
        let mut operation = None;
        let mut root = None;
        let mut units = Vec::new();
        let mut fetch_mode = vanllia::preparation::Mode::Locked;
        let mut offline = false;
        let mut artifact = None;
        let mut test_args = Vec::new();
        let mut changes = Vec::new();
        let mut manifest = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "usage: vanllia check|plan|build|run [--root DIRECTORY] [--unit ID ...]\n       vanllia build|run|test [--offline]\n       vanllia run --artifact DIRECTORY\n       vanllia test [--root DIRECTORY] [-- LIBTEST_ARGUMENTS...]\n       vanllia fetch [--root DIRECTORY] [--update]\n       vanllia workspace [--source NAME=PATH] [--reset-source NAME] [--manifest CARGO_TOML] [--offline]"
                    );
                    return Ok(None);
                }
                "--version" | "-V" => {
                    println!("vanllia {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                "--root" => {
                    root = Some(PathBuf::from(
                        args.next().ok_or("--root needs a directory")?,
                    ))
                }
                "--unit" => units.push(
                    args.next()
                        .filter(|id| !id.is_empty())
                        .ok_or("--unit needs a Unit ID")?,
                ),
                "check" if operation.is_none() => operation = Some(Operation::Check),
                "plan" if operation.is_none() => operation = Some(Operation::Plan),
                "fetch" if operation.is_none() => operation = Some(Operation::Fetch(fetch_mode)),
                "build" if operation.is_none() => {
                    operation = Some(Operation::Build(vanllia::build::Options::default()))
                }
                "run" if operation.is_none() => {
                    operation = Some(Operation::Run {
                        options: vanllia::build::Options::default(),
                        artifact: None,
                    })
                }
                "test" if operation.is_none() => {
                    operation = Some(Operation::Test {
                        offline: false,
                        args: Vec::new(),
                    })
                }
                "--artifact" => {
                    artifact = Some(PathBuf::from(
                        args.next()
                            .ok_or("--artifact needs an artifact directory")?,
                    ))
                }
                "--" => {
                    test_args.extend(args);
                    break;
                }
                "--offline" => offline = true,
                "workspace" if operation.is_none() => {
                    operation = Some(Operation::Workspace {
                        changes: Vec::new(),
                        manifest: None,
                        offline: false,
                    })
                }
                "--source" => {
                    let value = args.next().ok_or("--source needs NAME=PATH")?;
                    let (name, path) = value
                        .split_once('=')
                        .filter(|(name, path)| !name.is_empty() && !path.is_empty())
                        .ok_or("--source needs NAME=PATH")?;
                    changes.push(vanllia::workspace::Change::Use {
                        name: name.into(),
                        path: path.into(),
                    });
                }
                "--reset-source" => changes.push(vanllia::workspace::Change::Reset(
                    args.next().ok_or("--reset-source needs a source name")?,
                )),
                "--manifest" => {
                    manifest = Some(PathBuf::from(
                        args.next().ok_or("--manifest needs Cargo.toml")?,
                    ))
                }
                "--update" => fetch_mode = vanllia::preparation::Mode::Update,
                _ => return Err(format!("unknown argument `{arg}`; use vanllia --help")),
            }
        }
        let workspace_flags = !changes.is_empty() || manifest.is_some();
        let operation = match operation
            .ok_or("expected check, plan, fetch, build, run or test; use vanllia --help")?
        {
            Operation::Fetch(_) => Operation::Fetch(fetch_mode),
            Operation::Build(_) if fetch_mode == vanllia::preparation::Mode::Locked => {
                Operation::Build(vanllia::build::Options { offline })
            }
            Operation::Run { .. } if fetch_mode == vanllia::preparation::Mode::Locked => {
                Operation::Run {
                    options: vanllia::build::Options { offline },
                    artifact: artifact.clone(),
                }
            }
            Operation::Test { .. } if fetch_mode == vanllia::preparation::Mode::Locked => {
                Operation::Test {
                    offline,
                    args: test_args.clone(),
                }
            }
            Operation::Workspace { .. } if fetch_mode == vanllia::preparation::Mode::Locked => {
                Operation::Workspace {
                    changes,
                    manifest,
                    offline,
                }
            }
            other if fetch_mode == vanllia::preparation::Mode::Locked => other,
            _ => return Err("--update is only valid for fetch".into()),
        };
        if artifact.is_some() && !matches!(operation, Operation::Run { .. }) {
            return Err("--artifact is only valid for run".into());
        }
        if workspace_flags && !matches!(operation, Operation::Workspace { .. }) {
            return Err("--source/--reset-source/--manifest are only valid for workspace".into());
        }
        if !test_args.is_empty() && !matches!(operation, Operation::Test { .. }) {
            return Err("arguments after -- are only valid for test".into());
        }
        if offline
            && !matches!(
                operation,
                Operation::Build(_)
                    | Operation::Run { .. }
                    | Operation::Test { .. }
                    | Operation::Workspace { .. }
            )
        {
            return Err("--offline is only valid for build/run/test/workspace".into());
        }
        if !matches!(
            operation,
            Operation::Plan | Operation::Build(_) | Operation::Run { artifact: None, .. }
        ) && !units.is_empty()
        {
            return Err(
                "--unit is valid for plan/build/run; existing artifacts preserve their built selection".into(),
            );
        }
        Ok(Some(Self {
            operation,
            root: match root {
                Some(root) => root,
                None => std::env::current_dir().map_err(|error| error.to_string())?,
            },
            selection: if units.is_empty() {
                Selection::Default
            } else {
                Selection::Explicit(units)
            },
        }))
    }
}
