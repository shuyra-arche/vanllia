mod invocation;
mod operation;
mod test_command;

use invocation::Invocation;
use operation::Operation;
use vanllia::{Error, reason::Reason};

fn run(invocation: Invocation) -> Result<(), Error> {
    if let Operation::Workspace {
        changes,
        manifest,
        offline,
    } = &invocation.operation
    {
        let editor = vanllia::workspace::configure(
            &invocation.root,
            changes,
            manifest.as_deref(),
            *offline,
        )?;
        println!("workspace sources configured");
        if let Some(path) = editor {
            println!("editor settings: {}", path.display());
        }
        return Ok(());
    }
    if let Operation::Test { offline, args } = &invocation.operation {
        return test_command::run(&invocation.root, *offline, args);
    }
    if let Operation::Run { options, artifact } = &invocation.operation {
        let root = vanllia::discovery::find_root(&invocation.root)?;
        let project = vanllia::config::project(&root)?;
        let settings = project
            .runner
            .ok_or_else(|| Error::Workflow("run requires [runner]".into()))?;
        let receipt = match artifact {
            Some(path) => vanllia::runner::load(path)?,
            None => vanllia::build::build(&root, &invocation.selection, options)?,
        };
        let result = vanllia::runner::run(&root, &receipt, &settings)?;
        println!("run evidence: {}", result.directory.display());
        if result.status != 0 {
            return Err(Error::Workflow(format!("VM exited with {}", result.status)));
        }
        return Ok(());
    }
    if let Operation::Build(options) = &invocation.operation {
        let receipt = vanllia::build::build(&invocation.root, &invocation.selection, options)?;
        println!("built: {}", receipt.directory.display());
        println!(
            "initrd: {} ({})",
            receipt.initrd.path.display(),
            receipt.initrd.sha256
        );
        return Ok(());
    }
    if let Operation::Fetch(mode) = invocation.operation {
        let root = vanllia::discovery::find_root(&invocation.root)?;
        let project = vanllia::config::project(&root)?;
        project.validate(&root.join("vanllia.toml"))?;
        let sources = vanllia::preparation::prepare(&root, &project, mode)?;
        println!("prepared: {} declared sources", sources.len());
        return Ok(());
    }
    let catalog = vanllia::load(&invocation.root)?;
    match invocation.operation {
        Operation::Check => println!(
            "checked: {} Units, {} Cargo bins",
            catalog.units().len(),
            catalog.images().len()
        ),
        Operation::Plan => {
            let deployment = vanllia::plan(&catalog, &invocation.selection)?;
            println!("boot: {}", deployment.system);
            for id in &deployment.order {
                let declaration = &catalog.units()[id];
                let image = &catalog.images()[&declaration.unit.loader.image];
                let reasons = deployment.reasons[id]
                    .iter()
                    .map(|reason| match reason {
                        Reason::Boot => "System boot entry".into(),
                        Reason::Selected => "selected".into(),
                        Reason::RequiredBy(owner) => format!("required by {owner}"),
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                println!("unit: {id} ({reasons})");
                println!("  declaration: {}", declaration.path.display());
                println!(
                    "  image: {} / package {} / {}",
                    image.name,
                    image.package,
                    image.manifest.display()
                );
                let after: Vec<_> = declaration
                    .unit
                    .control
                    .after
                    .iter()
                    .filter(|id| deployment.reasons.contains_key(*id))
                    .collect();
                println!("  after: {after:?}");
            }
            println!("declaration order: {}", deployment.order.join(" -> "));
            let initial = deployment
                .order
                .iter()
                .filter(|id| {
                    catalog.units()[*id].unit.control.creation == vanllia::unit::Creation::Startup
                })
                .cloned()
                .collect::<Vec<_>>();
            println!("initial instances: {}", initial.join(", "));
        }
        Operation::Fetch(_)
        | Operation::Build(_)
        | Operation::Run { .. }
        | Operation::Test { .. }
        | Operation::Workspace { .. } => {
            unreachable!("mutation is handled before catalog discovery")
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = vanllia_build::workflow::process::initialize() {
        eprintln!("vanllia: {error}");
        std::process::exit(2);
    }
    if let Err(error) = ctrlc::set_handler(vanllia_build::workflow::cancel) {
        eprintln!("vanllia: cannot install cancellation handler: {error}");
        std::process::exit(2);
    }
    let result = Invocation::parse(std::env::args().skip(1))
        .map_err(|error| error.to_string())
        .and_then(|invocation| {
            invocation
                .map(run)
                .transpose()
                .map_err(|error| error.to_string())
        });
    if let Err(error) = result {
        eprintln!("vanllia: {error}");
        std::process::exit(2);
    }
}
