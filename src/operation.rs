pub(crate) enum Operation {
    Check,
    Plan,
    Fetch(vanllia::preparation::Mode),
    Build(vanllia::build::Options),
    Run {
        options: vanllia::build::Options,
        artifact: Option<std::path::PathBuf>,
    },
    Test {
        offline: bool,
        args: Vec<String>,
    },
    Workspace {
        changes: Vec<vanllia::workspace::Change>,
        manifest: Option<std::path::PathBuf>,
        offline: bool,
    },
}
