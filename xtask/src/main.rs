use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Parser)]
#[command(about = "Repository task runner")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    /// Build the WASM client, static pages, and Pagefind index.
    Build,
    /// Verify an existing production build without mutating it.
    Check,
    /// Generate or refresh only the Pagefind index.
    Index,
    /// Build the complete site and serve it locally.
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

fn run(command: &mut Command) -> Result<()> {
    let rendered = format!("{command:?}");
    let status = command
        .status()
        .with_context(|| format!("run child command {rendered}"))?;
    anyhow::ensure!(status.success(), "command failed with {status}: {rendered}");
    Ok(())
}

fn cargo(root: &Path) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(root);
    command
}

fn build_wasm(root: &Path) -> Result<()> {
    run(cargo(root).args([
        "build",
        "--package",
        "blog-client",
        "--target",
        "wasm32-unknown-unknown",
        "--release",
    ]))
}

fn generator(root: &Path, arguments: &[&str]) -> Result<()> {
    run(cargo(root)
        .args(["run", "--package", "blog-generator", "--"])
        .args(arguments))
}

fn pagefind_program(root: &Path) -> PathBuf {
    if let Some(program) = env::var_os("PAGEFIND") {
        return PathBuf::from(program);
    }
    let bundled = root.join(".tools/pagefind").join(if cfg!(windows) {
        "pagefind_extended.exe"
    } else {
        "pagefind_extended"
    });
    if bundled.is_file() {
        bundled
    } else {
        PathBuf::from(if cfg!(windows) {
            "pagefind_extended.exe"
        } else {
            "pagefind_extended"
        })
    }
}

fn index(root: &Path) -> Result<()> {
    let program = pagefind_program(root);
    let mut command = Command::new(&program);
    command.current_dir(root).args([
        "--site",
        "dist",
        "--output-subdir",
        "pagefind",
        "--force-language",
        "zh-cn",
    ]);
    if let Err(error) = run(&mut command) {
        bail!(
            "Pagefind indexing failed using {}. Set PAGEFIND or place the pinned standalone binary in .tools/pagefind: {error:#}",
            program.display()
        );
    }
    Ok(())
}

fn build(root: &Path) -> Result<()> {
    build_wasm(root)?;
    generator(root, &["build"])?;
    index(root)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = root()?;
    match cli.command {
        Task::Build => build(&root),
        Task::Check => generator(&root, &["check"]),
        Task::Index => index(&root),
        Task::Serve { port } => {
            build(&root)?;
            generator(&root, &["serve", "--port", &port.to_string()])
        }
    }
}
