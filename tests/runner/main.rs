use clap::{Parser, Subcommand};
use kernel_verification::Result;
use std::path::PathBuf;
mod native;
mod system;
mod versions;
#[derive(Parser)]
#[command(about = "Bounded native composition verification (host only)")]
struct Cli {
    #[command(subcommand)]
    scenario: Scenario,
}
#[derive(Subcommand)]
enum Scenario {
    Versions {
        #[arg(long)]
        fixture: PathBuf,
    },
    Native {
        #[arg(long)]
        kernel: PathBuf,
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value="debug", value_parser=["debug", "release"])]
        profile: String,
        #[arg(long, default_value_t=13, value_parser=clap::value_parser!(u8).range(10..=13))]
        capsule_version: u8,
    },
    System {
        #[arg(long)]
        kernel: PathBuf,
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        twine: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}
fn main() {
    let result: Result<()> = match Cli::parse().scenario {
        Scenario::Versions { fixture } => versions::run(&fixture),
        Scenario::Native {
            kernel,
            fixture,
            output,
            profile,
            capsule_version,
        } => native::run(&kernel, &fixture, &output, &profile, capsule_version),
        Scenario::System {
            kernel,
            fixture,
            twine,
            output,
        } => system::run(&kernel, &fixture, &twine, &output),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
