# Blender.rs
This library will help download, install, and render blender files. This library is treated as a wrapper class to invoke blender directly. This struct also handle downloading blender and check for version integrity by passing in executable path.

## Examples
I've composed a list of example what you can do with blender.rs

### Download
This example demonstrate downloading a copy of blender from the blender foundation organization, uncompressed the content, and return you a new struct containing blender path and version, ready to be used to render.

Run
```bash
cargo run --example download <version> [install_path]
# e.g.
cargo run --example download 4.1.0
```

For more info, please read [here](./examples/download/README.md).

### Render
This example will first check if you have blender installed, if not, it will ask you to run above examples. 