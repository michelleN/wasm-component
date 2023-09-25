use clap::{Parser, Subcommand};
use core::panic;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{self, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

fn inspect(wasm_path: &str) -> Result<(), anyhow::Error> {
    let p = path::Path::new(&wasm_path);
    if !p.exists() {
        panic!("Error: The file {} does not exist", wasm_path);
    }

    let tmp_dir = TempDir::new("wasm-component")?;
    let guest_path = &tmp_dir.path().join("guest.wit");
    let wit_path = guest_path.to_str().unwrap();

    generate_wit_file_from_wasm(wit_path, wasm_path)?;
    flip_wit(wit_path).unwrap();

    let world_name: String = get_world_name(wasm_path)?;

    generate_bindings(wit_path, &tmp_dir)?;

    generate_cargo_toml(tmp_dir.path().join("Cargo.toml"), world_name.clone());

    // TODO: hide/redirect stdout, hide deps?
    let child = Command::new("cargo")
        .args([
            "doc",
            "--manifest-path",
            tmp_dir.path().join("Cargo.toml").to_str().unwrap(),
            "--open",
        ])
        .spawn()
        .expect("Failed to gen docs");

    if child
        .wait_with_output()
        .expect("error generating docs")
        .status
        .success()
    {
        println!(
            "Documentation at {}",
            tmp_dir
                .path()
                .join(format!("target/doc/{}/index.html", world_name.clone()))
                .to_str()
                .unwrap()
        );
    } else {
        panic!("error generating docs")
    };

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

fn generate_cargo_toml(filepath: PathBuf, world_name: String) {
    let mut file = File::create(filepath.clone())
        .expect(format!("Unable to create file {:?}", filepath).as_str());

    // TODO: package name same as world name
    file.write_all(
        format!(
            r#"[package]
                name = "{}"
                version = "0.1.0"
                edition = "2021"

                [lib]
                path = "src/{}.rs"

                [dependencies]
                wit-bindgen = "0.11.0"
        "#,
            world_name, world_name
        )
        .as_bytes(),
    )
    .expect("error writing Cargo toml");
}

// This function reads the wit file at the given path. It removes any
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

fn generate_wit_file_from_wasm(wit_path: &str, wasm_path: &str) -> Result<(), anyhow::Error> {
    let mut wit_out = Command::new("wasm-tools")
        .args(["component", "wit", wasm_path, "-o", wit_path])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run the binary");
    wit_out.wait().unwrap();

    Ok(())
}

fn generate_bindings(wit_path: &str, tmp_dir: &TempDir) -> Result<(), anyhow::Error> {
    let mut gen_out = Command::new("wit-bindgen")
        .args([
            "rust",
            wit_path,
            "--out-dir",
            tmp_dir.path().join("src").to_str().unwrap(),
        ])
        .spawn()
        .expect("Failed to run the binary");
    gen_out.wait()?;
    Ok(())
}

fn get_world_name(wasm_path: &str) -> Result<String, anyhow::Error> {
    let out = Command::new("wasm-tools")
        .args(["component", "wit", wasm_path, "--json"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run the binary");
    let json_out = out.wait_with_output().unwrap();
    let parsed_json: Result<serde_json::Value, serde_json::Error> = if json_out.status.success() {
        let stdout = String::from_utf8_lossy(&json_out.stdout);
        serde_json::from_str(&stdout)
    } else {
        panic!("error here")
    };
    let parsed = parsed_json?;
    let world_name = match parsed
        .get("worlds")
        .and_then(|a| a.as_array())
        .and_then(|x| x.get(0))
        .and_then(|a| a.get("name"))
        .and_then(|a| a.as_str())
    {
        Some(n) => n.to_string(),
        None => panic!("cant find world name"),
    };
    Ok(world_name)
}
