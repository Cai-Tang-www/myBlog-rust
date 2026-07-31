use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "blog", about = "Build and preview the Rust myBlog rewrite")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build the static site into dist/.
    Build,
    /// Validate content and generated output.
    Check,
    /// Serve dist/ locally after building it.
    Serve {
        #[arg(long, default_value_t = 4173)]
        port: u16,
    },
}

fn workspace_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .context("resolve workspace root")
}

fn build_placeholder() -> Result<PathBuf> {
    let root = workspace_root()?;
    let dist = root.join("dist");
    fs::create_dir_all(&dist).context("create dist directory")?;
    fs::write(
        dist.join("index.html"),
        "<!doctype html><html lang=\"zh-CN\"><meta charset=\"utf-8\"><title>myBlog Rust</title><body><main><h1>myBlog Rust</h1><p>Rust static generator is ready.</p></main></body></html>\n",
    )
    .context("write placeholder index")?;
    Ok(dist)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build => {
            let dist = build_placeholder()?;
            println!("built {}", dist.display());
        }
        Command::Check => {
            let dist = build_placeholder()?;
            anyhow::ensure!(
                dist.join("index.html").is_file(),
                "missing generated index.html"
            );
            println!("checked {}", dist.display());
        }
        Command::Serve { port } => {
            let dist = build_placeholder()?;
            let address = format!("127.0.0.1:{port}");
            let server = tiny_http::Server::http(&address)
                .map_err(|error| anyhow::anyhow!("start preview server: {error}"))?;
            println!("serving {} at http://{address}", dist.display());
            for request in server.incoming_requests() {
                let path = request.url().trim_start_matches('/');
                let relative = if path.is_empty() { "index.html" } else { path };
                let candidate = dist.join(relative);
                let file = if candidate.is_dir() {
                    candidate.join("index.html")
                } else {
                    candidate
                };
                if file.is_file() {
                    let bytes = fs::read(&file)?;
                    let mime = mime_guess::from_path(&file).first_or_octet_stream();
                    let header = tiny_http::Header::from_bytes("Content-Type", mime.as_ref())
                        .map_err(|_| anyhow::anyhow!("invalid content type header"))?;
                    request.respond(tiny_http::Response::from_data(bytes).with_header(header))?;
                } else {
                    request.respond(tiny_http::Response::empty(404))?;
                }
            }
        }
    }
    Ok(())
}
