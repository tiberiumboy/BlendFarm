// an actor will act as a polling process for any IPC application
// whether it's Tauri application running frontend interfaces
// or the network interfaces and handling events across other network nodes
// Or even from blender itself

pub trait Actor {
    fn run(&self) -> Result<(), Box<dyn Error>>;
    fn abort(&self) -> Result<(), Box<dyn Error>>;
}
