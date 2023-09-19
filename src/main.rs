use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{self, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tempdir::TempDir;

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
}

fn inspect(path: &str) -> Result<(), std::io::Error> {
    let p = path::Path::new(&path);

    if !p.exists() {
        panic!("Error: The file {} does not exist", path);
    }

    let tmp_dir = TempDir::new("example")?;
    let file_path = &tmp_dir.path().join("guest.wit");

    Command::new("wasm-tools")
        .args(["component", "wit", path, "-o", file_path.to_str().unwrap()])
        .spawn()
        .expect("Failed to run the binary");

    sleep(Duration::new(3, 0)); // TODO

    flip_wit(file_path.to_str().unwrap()).unwrap();

    // TODO: really need name of the world at this point
    Command::new("wit-bindgen")
        .args([
            "rust",
            file_path.to_str().unwrap(),
            "--out-dir",
            tmp_dir.path().join("src").to_str().unwrap(),
        ])
        .spawn()
        .expect("Failed to run the binary");

    generate_cargo_toml(tmp_dir.path().join("Cargo.toml"));

    // TODO: hide/redirect stdout
    let mut child = Command::new("cargo")
        .args([
            "doc",
            "--manifest-path",
            tmp_dir.path().join("Cargo.toml").to_str().unwrap(),
            // "--open",
        ])
        .spawn()
        .expect("Failed to gen docs");

    child.wait()?;

    println!(
        "Documentation at {}",
        tmp_dir
            .path()
            .join("target/doc/root/index.html")
            .to_str()
            .unwrap()
    );

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst); // TODO: figure out what this does (from example)
        println!("received Ctrl+C!");
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {}
    Ok(())
}

fn generate_cargo_toml(filepath: PathBuf) {
    let mut file = File::create(filepath.clone())
        .expect(format!("Unable to create file {:?}", filepath).as_str());

    // TODO: package name same as world name
    file.write_all(
        r#"[package]
                name = "root"
                version = "0.1.0"
                edition = "2021"

                [lib]
                path = "src/root.rs"

                [dependencies]
                wit-bindgen = "0.11.0"
        "#
        .as_bytes(),
    )
    .unwrap();
}
// This function reads th wit file at the given path. It removes any
// exports and replaces them with imports. It deletes any lines that are imports.
// It then writes the new wit at a new path.
// TODO: In the future, we probably want to parse the wit file and do more sophisticated
// transformations. For example, we'd want to remove not only the import line but also
// any other types related to that import that are not being used by the exports.
fn flip_wit(wit_path: &str) -> Result<(), std::io::Error> {
    let export_keyword = "export";
    let import_keyword = "import";

    let file = match std::fs::File::open(wit_path) {
        Ok(file) => file,
        Err(err) => {
            panic!("Error: Unable to open the file {}: {}", wit_path, err);
        }
    };

    let new_lines: Vec<String> = BufReader::new(&file)
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            words.get(0) != Some(&import_keyword)
        })
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.get(0) == Some(&export_keyword) {
                line.replace(export_keyword, import_keyword)
            } else {
                line
            }
        })
        .collect();

    let output = new_lines.join("\n");

    let mut new_file = std::fs::File::create(wit_path)?;

    new_file
        .write_all(output.as_bytes())
        .expect("Unable to write data");

    Ok(())
}
