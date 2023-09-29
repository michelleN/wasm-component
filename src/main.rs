use clap::{Parser, Subcommand};
use core::panic;
use std::env::current_dir;
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
    command: Option<Commands>, // TODO remove option
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect the wit file of a component
    Inspect(Inspect),
}

/// Inspect given wasm file
#[derive(Parser, Debug)]
pub struct Inspect {
    /// Path to wit file
    path: String,

    /// Language to generate docs for bindings
    #[clap(long = "lang", short = 'l', value_parser = parse_lang)]
    pub language: String,
}

fn parse_lang(name: &str) -> Result<String, String> {
    match name.to_lowercase().as_str() {
        "rust" => Ok(name.to_owned()),
        "python" => Ok(name.to_owned()),
        _ => Err("Following languages are currently supported: rust, python".to_owned()),
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Inspect(i)) => {
            i.run().unwrap();
        }
        None => {}
    }
}

impl Inspect {
    pub fn run(&self) -> Result<(), anyhow::Error> {
        let wasm_path = &self.path;
        let p = path::Path::new(&wasm_path);
        if !p.exists() {
            panic!("Error: The file {} does not exist", wasm_path);
        }

        let tmp_dir = TempDir::new("wasm-component")?.into_path();
        let guest_path = &tmp_dir.as_path().join("guest.wit");
        let wit_path = guest_path.to_str().unwrap();

        generate_wit_file_from_wasm(wit_path, wasm_path)?;
        // TODO: if lang is not provided, return wit file here

        flip_wit(wit_path).unwrap();

        let world_name: String = get_world_name(wasm_path)?;

        generate_bindings(wit_path, &self.language, &world_name, &tmp_dir)?;

        generate_docs(&self.language, &world_name, &tmp_dir);

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
}

fn generate_docs(lang: &str, world_name: &str, tmp_dir: &PathBuf) {
    match lang {
        "rust" => generate_rust_docs(world_name, tmp_dir),
        "python" => generate_python_docs(world_name, tmp_dir),
        _ => panic!("Following languages are currently supported: rust, python"),
    }
}

fn generate_python_docs(world_name: &str, tmp_dir: &PathBuf) {
    let child = Command::new("pydoctor")
        .args([
            "--make-html",
            "--html-output",
            tmp_dir.as_path().join("docs").to_str().unwrap(),
            tmp_dir.as_path().join(world_name).to_str().unwrap(),
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
            "\n\n\nDocumentation is at {}",
            tmp_dir
                .as_path()
                .join("docs")
                .join("index.html")
                .to_str()
                .unwrap(),
        );
    } else {
        panic!("error generating docs")
    };
}

fn generate_rust_docs(world_name: &str, tmp_dir: &PathBuf) {
    generate_cargo_toml(tmp_dir.as_path().join("Cargo.toml"), world_name.to_string());

    // TODO: hide/redirect stdout, hide deps?
    let child = Command::new("cargo")
        .args([
            "doc",
            "--manifest-path",
            tmp_dir.as_path().join("Cargo.toml").to_str().unwrap(),
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
            "\n\n\nDocumentation at {}",
            tmp_dir
                .as_path()
                .join(format!("target/doc/{}/index.html", world_name.clone()))
                .to_str()
                .unwrap()
        );
    } else {
        panic!("error generating docs")
    };
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

fn generate_rust_bindings(wit_path: &str, tmp_dir: &PathBuf) -> Result<(), anyhow::Error> {
    let mut gen_out = Command::new("wit-bindgen")
        .args([
            "rust",
            wit_path,
            "--out-dir",
            tmp_dir.as_path().join("src").to_str().unwrap(),
        ])
        .spawn()
        .expect("Failed to run the binary");
    gen_out.wait()?;
    Ok(())
}

fn generate_python_bindings(
    wit_path: &str,
    world_name: &str,
    tmp_dir: &PathBuf,
) -> Result<(), anyhow::Error> {
    let mut gen_out = Command::new("componentize-py")
        .args([
            "-d",
            wit_path,
            "-w",
            world_name,
            "bindings",
            // tmp_dir.as_path().join(world_name).to_str().unwrap(),
            tmp_dir.as_path().to_str().unwrap(),
        ])
        .spawn()
        .expect("Failed to run the binary");
    gen_out.wait().unwrap();

    Ok(())
}

fn generate_bindings(
    wit_path: &str,
    lang: &str,
    world_name: &str,
    tmp_dir: &PathBuf,
) -> Result<(), anyhow::Error> {
    match lang {
        "rust" => generate_rust_bindings(wit_path, tmp_dir),
        "python" => generate_python_bindings(wit_path, world_name, tmp_dir),
        _ => panic!("Following languages are currently supported: rust, python"),
    }
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
