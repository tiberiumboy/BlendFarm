// here we'll provide basic cli interface controls to list, edit, add, or remove blender installations history.
// Below the surface should follow simple implementations similar to REST api.

use blender::{blender::get_blend_config_from_local, blender::Blender, manager::Manager};
use std::path::PathBuf;
// TODO: I only want to use clap for examples, but not include with the whole library itself.
use clap::{Parser, Subcommand};
use semver::Version;

#[derive(Subcommand, Debug)]
enum Command {
    Add { path: PathBuf },
    ExactDownload { version: Version },
    // minor can accept 0 as default (Wildcard to use latest)
    Download { major: u64, minor: u64 },
    // Disconnect { target: Version },
    // Delete { target: Version},
}

/// The manager cli is a great way to interface the persistent manager state for BlendFarm services.
/// This manager responsibility is to fetch and download (Portal), unpack and install (Config) Blenders installation.
/// This struct stores a collection of executable path to blender, and holds the version as unique key identifier.
/// The manager only cares about single instance version of blender that is bound to the software version.
///
/// Caller can invoke built-in commands to update the persistent storage to include locally installed blender
/// to the list of available blender installations for BlendFarm to use from.
/// You can also run commands to download and install specific or latest blender version online.
/// ```
/// cargo run # Returns list of known configurable blender installation path.
///
/// cargo run -- add "./path/to/blender" # Verify executable and append to Manager's collection of installations.
///
/// cargo run -- exact-download "4.2.4"
/// ```
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to load custom config location, otherwise load from default location (~/.config/BlendFarm/BlenderManager.json)
    config: Option<PathBuf>,
    /// Subcommand to invoke cli utility mode
    #[command(subcommand)]
    command: Option<Command>,
}

fn handle_download_blender(manager: &mut Manager, version: &Version) {
    match manager.fetch_blender(&version) {
        Ok(blender) => println!(
            "[Success] Blender {} installed at {:?}",
            blender.get_version(),
            blender.get_executable()
        ),
        Err(e) => eprintln!("[Fail] Unable to fetch blender {}: {:?}", &version, e),
    }
    // you should at least save the record if it has been modified. Otherwise all record changes will not be saved away.
    if let Err(e) = manager.save() {
        eprintln!("Unable to update persistent data! Changes made will be lost! {e:?}");
    }
}

fn main() {
    // retrieve the sub command the user wants to invoke
    // let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    let args = Args::parse();

    let config = get_blend_config_from_local().expect("Must have blender config to continue!");
    let mut manager = Manager::load(config).expect(&format!(
        "Unable to launch manager, must have valid config!"
    ));

    // find a way to accept "add" "edit" "delete" blender collection. Modify and save the list verbosely.
    match args.command {
        Some(action) => match action {
            Command::Add { path } => {
                let blender = Blender::from_executable(path)
                    .expect("Path must point to valid blender executable location!");
                if let Err(e) = manager.add_blender(&blender) {
                    eprintln!("Fail to add blender! {e:?}");
                }
                if let Err(e) = manager.save() {
                    eprintln!("Unable to update existing config file! {e:?}");
                }
            }
            Command::ExactDownload { version } => {
                handle_download_blender(&mut manager, &version);
            }
            // Download exact version from the internet.
            Command::Download { major, minor } => {
                // the secret trick is to use patch 0 to use the latest version available.
                let version = Version::new(major, minor, 0);
                handle_download_blender(&mut manager, &version);
                // Here we will try and download blender from the internet.
            } // Command::Disconnect { target } => {
              //     todo!("We'll come back to this one... This one a bit weird and odd...");
              // },
              // Command::Delete { target } => todo!(),
        },
        None => manager
            .get_config()
            .get_blenders()
            .iter()
            .for_each(|v| println!("{v:?}")),
    }
}
