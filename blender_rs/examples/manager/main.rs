// here we'll provide basic cli interface controls to list, edit, add, or remove blender installations history.
// Below the surface should follow simple implementations similar to REST api.

// todo, load the config file here.

use std::path::PathBuf;

use blender::{manager::Manager, models::blender_config::BlenderConfig};

fn main() {
    // retrieve the sub command the user wants to invoke
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    // see about getting subcommands
    let config_path = match args.get(1) {
        // FIXME: Path is relative to where command is invoked. Must be from blender_rs directory, otherwise path will fail.
        None => BlenderConfig::get_default_config_path(),
        Some(p) => PathBuf::from(p),
    };

    let manager = Manager::load(&config_path).expect(&format!("Unable to launch manager, must have valid config! {config_path:?}"));

    // default would to list out current blender info.
    manager.get_blenders().iter().for_each(|v| println!("{v:?}"));
}