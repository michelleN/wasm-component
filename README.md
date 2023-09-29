# Wasm Component

This tool helps inspect a WASM component so that you can determine if it is something you can or want to use in your application.

For now, you'll have to download or create a wasm component (.wasm file) and use this tool with a path to the file on the local filesystem. In the future, we'll add registry support so you can search from a registry.

This tool is a prototype to understand what experience helps people search, discover and use components in their applications quickly. This tool might be okay to use in the near term and may not be necessary in the future when the warg ecosystem is more mature or if/when language ecosystems make it easier to work with components.

## Workflow

1) Download a wasm component or use the one included in this repository.
2) Run the inspect command like below:

```bash
# create component at path compose.wasm or use the compose.wasm included in this repo

$ cargo run inspect compose.wasm --lang rust
```
This command will open up rust documentation for this component in the browser. Ctrl-c will exit and delete all generated docs.

_Note: There is also support for python using pydoctor under the hood but it doesn't automatically open up your broswer. You'll need to click the link provided in your terminal_


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
