use std::{sync::Arc};

use log::info;
use simple_reactor::{
    Client, SocketRemote, UdpSocket
};
use std::io::{self, Write};

fn datagram_callback(
    _remote: Arc<SocketRemote<UdpSocket>>,
    data: &mut [u8],
    addr: std::net::SocketAddr,
    _receive_time: std::time::Instant,
) {
    let content = String::from_utf8_lossy(data);
    println!(
        "Received datagram from {}: {}",
        addr,
        content
    );
}

fn main() {
    env_logger::Builder::from_default_env()
        // .filter_level(log::LevelFilter::Info)
        .init();
    info!("env_logger inited");

    let addr = "127.0.0.1:0".to_string();
    let server = "127.0.0.1:8888".to_string();
    let mut client = Client::<UdpSocket>::new(
        mio::net::UdpSocket::bind(addr.clone().parse().unwrap()).expect("Failed to bind UDP socket"),
        Arc::new(datagram_callback),
    );
    client.listen();

    loop {
        let mut input = String::new();
        print!("请输入要发送的内容: ");
        io::stdout().flush().unwrap();
        if io::stdin().read_line(&mut input).is_ok() {
            if input.starts_with("exit") {
                break;
            }
            let bytes = input.trim_end().as_bytes();
            client.send(server.clone().parse().unwrap(), bytes);
        }
    }

    client.shutdown();
}
