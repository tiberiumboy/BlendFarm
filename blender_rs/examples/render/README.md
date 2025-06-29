# Render
This section includes example to render a example `test.blend` scene file. 
To run, pass in the following argument: This will use whatever latest blender you have installed in your server settings, and render the default scene, or unless override with the optional argument, your blender scene file.
```bash
cargo run --example render [path/to.blend]
# e.g. to render test.blend inside /examples/render/ folder - call the following line:
cargo run --example render
```
