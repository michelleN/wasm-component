# Wasm Component

This tool aims to help with search and discovery of WASM components. For now, you'll have to download or create a wasm component (.wasm file) and use this tool with a path to the file on the local filesystem. In the future, we'll add registry support so you can search from a registry.

This tool is a prototype to understand what experience helps people search, discover and use components in their applications quickly. This tool might be okay to use in the near term and may not be necessary in the future when the warg ecosystem is more mature or if/when language ecosystems make it easier to work with components.

```bash
# create component at path compose.wasm or use the compose.wasm included in this repo

$ cargo run inspect compose.wasm
```

## Generate a test .wasm from a WIT

```bash
wasm-tools component wit <your-wit>.wit -w -o <your-component>.wasm
```

Then use the new .wasm file: `wasm-component inspect <your-component>.wasm`

## Build 

```bash
$ git clone git@github.com:michelleN/wasm-component.git
$ cd wasm-component
$ cargo build --release
```

or build and install wasm-component on your path:

```
cargo install --path . --locked
```
