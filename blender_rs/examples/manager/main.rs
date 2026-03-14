// here we'll provide basic cli interface controls to list, edit, add, or remove blender installations history.
// Below the surface should follow simple implementations similar to REST api.

// todo, load the config file here.

use std::path::PathBuf;

use blender::manager::Manager;

fn main() {
    // retrieve the sub command the user wants to invoke
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    // see about getting subcommands
    let config_path = args.get(1).map(PathBuf::from);
    let manager = Manager::load(config_path).expect(&format!("Unable to launch manager, must have valid config!"));

    // default would to list out current blender info.
    manager.get_blenders().iter().for_each(|v| println!("{v:?}"));
}