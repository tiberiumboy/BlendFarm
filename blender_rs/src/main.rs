use std::env::current_dir;

fn main() {
    if let Ok(path) = current_dir() {
        let project_path = path.to_string_lossy();
        println!("Please read the example to learn more about Blender crate - ${}/examples/render/README.md", project_path);
    }
}
