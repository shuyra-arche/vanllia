use std::path::Path;
use vanllia::Error;

pub(crate) fn run(start: &Path, offline: bool, args: &[String]) -> Result<(), Error> {
    let root = vanllia::discovery::find_root(start)?;
    let mut arguments = libtest_mimic::Arguments::from_iter(
        std::iter::once("vanllia test".to_string()).chain(args.iter().cloned()),
    );
    if arguments.test_threads.is_some_and(|threads| threads > 1) {
        return Err(Error::Workflow(
            "tests currently share one Cargo resolution context; use --test-threads=1".into(),
        ));
    }
    arguments.test_threads = Some(1);
    if !offline && !arguments.list {
        let project = vanllia::config::project(&root)?;
        vanllia::preparation::prepare(&root, &project, vanllia::preparation::Mode::Locked)?;
    }
    let cases = vanllia::testing::cases(&root)?;
    if cases.is_empty() {
        return Err(Error::Workflow(
            "no test.toml found in the selected source scope".into(),
        ));
    }
    let tests = cases
        .into_iter()
        .map(|case| {
            let root = root.clone();
            libtest_mimic::Trial::test(case.name.clone(), move || {
                vanllia::testing::execute(&root, &case, offline)
                    .map_err(|why| why.to_string().into())
            })
        })
        .collect();
    let conclusion = libtest_mimic::run(&arguments, tests);
    if conclusion.has_failed() {
        return Err(Error::Workflow("integration tests failed".into()));
    }
    Ok(())
}
