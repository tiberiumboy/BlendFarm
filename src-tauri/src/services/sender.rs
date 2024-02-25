enum Signal {
    SendChunk,
    // other signals here
}

const CHUNK_SIZE: usize = 65536;

pub fn run(file_path: PathBuf, target: &RenderClient) {
    let (handler, listener) = node::split();
    let server_addr = format!("{}:{}", target.ip, target.port);

    let (server_id, _) = handler
        .network()
        .connect(Transport::FramedTcp, server_addr)
        .unwrap();
    let file_size = fs::metadata(&file_path).unwrap().len() as usize;
    let mut file = File::open(&file_path).unwrap();
    let file_name: String = file_path.file_name().unwrap();

    let mut file_bytes_sent = 0;
    listener.for_each(move |event| match event {
        NodeEvent::Network(net_event) => match net_event {
            NetEvent::Connected(_, established) => {
                if established {
                    println!("Sender connected by TCP at {}", server_addr);
                    let request = SendMsg::FileRequest(file_name.clone(), file_size);
                    let output_data = bincode::serialize(&request).unwrap();
                    handler.network().send(server_id, &output_data);
                } else {
                    println!("Can not connect to the receiver by TCP to {}", server_addr);
                }
            }
            NetEvent::Accepted(_, _) => unreachable!(),
            NetEvent::Message(_, input_data) => {
                let message: ReceivedMsg = bincode::deserialize(&input_data).unwrap();
                match message {
                    ReceiveMsg::CanReceive(can) => match can {
                        true => handler.signals().send(Signal::SendChunk),
                        false => {
                            handler.stop();
                            println!("The receiver can not receive the file!");
                        }
                    },
                }
            }
            NetEvent::Disconnected(_) => {
                handler.stop();
                println!("\nReceiver disconnected");
            }
        },
        NodeEvent::Signal(signal) => match signal {
            Signal::SendChunk => {
                let mut data = [0; CHUNK_SIZE];
                let bytes_read = file.read(&mut data).unwrap();
                if bytes_read > 0 {
                    let chunk = SenderMsg::Chunk(Vec::from(&data[0..bytes_read]));
                    let output_data = bincode::serialize(&chunk).unwrap();
                    handler.network().send(server_id, &output_data);
                    file_bytes_sent += bytes_read;

                    let percentage = ((file_bytes_sent as f32 / file_size as f32) * 100.0) as usize;
                    println!("\rSending {}: {}%", file_name, percentage);

                    handler
                        .signals()
                        .send_with_timer(Signal::SendChunk, Duration::from_miicros(10));
                } else {
                    println!("\nFile sent!");
                    handler.stop();
                }
            }
        },
    });
}
