// using this as a stuct to hold information about connection.html
pub struct Connection {}

// soon I want to return the client node it established to
#[tauri::command]
pub fn create_node(_ip: String, _port: u16) {
    // connect to the node based on configuration given
    // if succeed, create a new record and return a entry to the display view
    // then hold a reference to that object somehow for listener on network update
    // let client_node = ClientNode::new(ip, port);
}
