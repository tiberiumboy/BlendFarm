// use std::io::{self, Read, Result};
use std::net::Ipv4Addr; //, SocketAddr, TcpStream, ToSocketAddrs};

struct RenderClient {
    pub ip: Ipv4Addr,
    pub port: u16,
}

impl RenderClient {
    // fn extract_string(buf: &mut impl Read) -> io::Result<String> {
    //     let len = buf.bytes().count();
    //     let mut bytes = vec![0u8, len as u8];
    //     buf.read_exact(&mut bytes)?;

    //     String::from_utf8(bytes)
    //         .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid utf8"))
    // }

    fn new(ip: Ipv4Addr, port: u16) -> Self {
        RenderClient { ip, port }
    }

    // fn send(&self, message: Packet) -> io::Result<()> {
    //     let mut stream = TcpStream::connect((self.ip, self.port))?;

    //     let bytes_written = stream.write(message.to_byte())?;

    //     if bytes_written < message.len() {
    //         return Err(io::Error::new(
    //             io::ErrorKind::Interrupted,
    //             format!("Send {}/{} bytes", bytes_written, message.len()),
    //         ));
    //     }

    //     stream.flush()?;
    //     Ok(())
    // }

    // fn read(&self) -> io::Result<()> {
    //     let mut stream = TcpStream::connect(&self.ip)?;
    //     stream
    //         .set_nonblocking(true)
    //         .expect("set_nonblocking call failed!");

    //     let mut buf = vec![];
    //     loop {
    //         match stream.read_to_end(&mut buf) {
    //             Ok(_) => break,
    //             Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
    //                 // wait_for_fd();
    //             }
    //             Err(e) => panic!("encountered IO error: {e}"),
    //         };
    //     }
    //     Ok(())
    // }
}
