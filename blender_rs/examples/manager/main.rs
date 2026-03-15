// here we'll provide basic cli interface controls to list, edit, add, or remove blender installations history.
// Below the surface should follow simple implementations similar to REST api.

// todo, load the config file here.

use std::path::PathBuf;
// TODO: I only want to use clap for examples, but not include with the whole library itself.
use clap::{Parser, Subcommand};

use blender::{blender::Blender, manager::Manager};
// use semver::Version;

#[derive(Subcommand, Debug)]
enum Command {
    Add { path: PathBuf },
    // Disconnect { target: Version },
    // Delete { target: Version},
}

#[derive(Parser, Debug)]
struct Args {
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>
}

fn main() {
    // retrieve the sub command the user wants to invoke
    // let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    let args = Args::parse();
    let mut manager = Manager::load(args.config).expect(&format!("Unable to launch manager, must have valid config!"));

    // find a way to accept "add" "edit" "delete" blender collection. Modify and save the list verbosely.
    match args.command {
        Some(action) => match action {
            Command::Add { path } => {
                let blender = Blender::from_executable(path).expect("Path must point to valid blender executable location!");
                if let Err(e) = manager.add_blender(&blender) {
                    eprintln!("Fail to add blender! {e:?}");
                }
                if let Err(e) = manager.save() {
                    eprintln!("Unable to update existing config file! {e:?}");
                }
            },
            // Command::Disconnect { target } => {
            //     todo!("We'll come back to this one... This one a bit weird and odd...");
            // },
            // Command::Delete { target } => todo!(),
        },
        None => manager.get_blenders().iter().for_each(|v| println!("{v:?}")),
    }
}