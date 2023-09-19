# Wasm Component

This tool aims to help with search and discovery of WASM components. For now, you'll have to download or create a wasm component (.wasm file) and use this tool with a path to the file on the local filesystem. In the future, we'll add registry support so you can search from a registry.

This tool is a prototype to understand what experience helps people search, discover and use components in their applications quickly. This tool might be okay to use in the near term and may not be necessary in the future when the warg ecosystem is more mature or if/when language ecosystems make it easier to work with components.

```bash
# create component at path compose.wasm

$ cargo run inspect compose.wasm
```
