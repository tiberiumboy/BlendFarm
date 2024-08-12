use clap::{command, Parser};

#[derive(Parser)]
#[command(name = "BlenderFarm")]
#[command(version = "0.1.0")]
#[command(
    about = "BlenderFarm is a distributed rendering system that allows users to render blender files on multiple machines."
)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(short, long)]
    #[arg(help = "Run the application as a rendering node")]
    client: bool,
}

impl Cli {
    pub fn is_client(&self) -> bool {
        self.client
    }
}
