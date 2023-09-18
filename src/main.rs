use clap::{Parser, Subcommand};
use std::path;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect the wit file of a component
    Inspect { path: String },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Inspect { path }) => {
            inspect(path).unwrap();
        }
        None => {}
    }

    // Continued program logic goes here...
}

fn inspect(path: &str) -> Result<(), std::io::Error> {
    let p = path::Path::new(&path);

    // Check that file exists
    std::fs::File::open(p).unwrap();

    Command::new("wasm-tools")
        .args(["component", "wit", path, "--output=guest.wit"])
        .spawn()
        .expect("Failed to run the binary");

    Command::new("wit-bindgen")
        .args(["rust", "guest.wit", "--output=src/wit.rs"])
        .spawn()
        .expect("Failed to run the binary");

    // TODO: the third piece of this is to wire up cargo doc to make the generated bindings cute

    Ok(())
}
