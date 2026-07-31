use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, process::Command};

#[derive(Debug, Parser)]
#[command(about = "Repository task runner")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    Build,
    Check,
    Serve {
        #[arg(long, default_value_t = 4173)]
        port: u16,
    },
}

fn root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .context("resolve workspace root")
}

fn run(mut command: Command) -> Result<()> {
    let status = command.status().context("run child command")?;
    anyhow::ensure!(status.success(), "command failed with {status}");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = root()?;
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["run", "--package", "blog-generator", "--"]);
    match cli.command {
        Task::Build => command.arg("build"),
        Task::Check => command.arg("check"),
        Task::Serve { port } => command.args(["serve", "--port", &port.to_string()]),
    };
    run(command)
}
