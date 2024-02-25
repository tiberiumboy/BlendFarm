use semver::Version;
use std::io::{self, BufRead};
use std::net::{IpAddr, Ipv4Adr, Ipv6Addr, TcpStream};

enum RenderNodeStatus {
    Error,
    Idle,
    Active,
    Downloading,
    Loading,
}

struct RenderNode {
    pub version: Version, // why do I need this? I'm curious?
    pub stream: TcpStream,
    // do I need to care about logs? They're printed on the console anyway...?
    // currentLog : Vec<String>,
    pub name: String,
    pub stream: TcpStream,
    //
    pub status: RenderNodeStatus, // could make this as an enum?
    // do I care about the os?
    pub os: String,
    pub pass: String, // I swear to god this better not be the password...
    pub client: RenderClient,
}

impl RenderNode {
    fn send(&self, msg: BlendFarmMessage) -> std::io::Result<()> {
        // should we check and see if we're still connected? or nah?
        self.stream.write_all(msg.to_byte())?;
        self.stream.flush()
    } // stream closes here

    fn read(&self) -> std::io::Result<()> {
        let mut reader = io::BufReader::new(&mut self.stream);
        let received: Vec<u8> = reader.fill_buf()?.to_vec();

        reader.consume(received.len());

        String::from_utf8(received)
            .map(|msg| println!("{}", msg))
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Couldn't parse receiving string as utf8",
                )
            });
    }

    fn new(name: str, addr: IpAddr) -> RenderNode {
        RenderNode {
            version: Version::new(),
            stream: TcpStream::connect(addr.parse()),
            address: addr,
            name: name,
        }
    }
}
