use log::info;
use simple_reactor::{Buffer, Server, SocketRemote, TcpConnection, UdpSocket};
use std::sync::Arc;

fn message_callback(
    remote: Arc<SocketRemote<TcpConnection>>,
    buffer: &mut Buffer,
    _receive_time: std::time::Instant,
) {
    let content = buffer.as_slice().to_vec();
    buffer.retrieve_all();
    info!(
        "Received message from {}: {:?}",
        remote.local_addr(),
        String::from_utf8_lossy(&content),
    );
    remote.write(&content);
}

fn connection_callback(remote: Arc<SocketRemote<TcpConnection>>, is_connected: bool) {
    if is_connected {
        info!("New connection established: {}", remote.peer_addr());
    } else {
        info!("Connection closed: {}", remote.peer_addr());
    }
}

fn datagram_callback(
    remote: Arc<SocketRemote<UdpSocket>>,
    data: &mut [u8],
    addr: std::net::SocketAddr,
    _receive_time: std::time::Instant,
) {
    let content = String::from_utf8_lossy(data);
    info!(
        "Received datagram from {}: {}",
        addr,
        content
    );
    remote.send(addr, data);
}

fn main() {
    env_logger::Builder::from_default_env()
        // .filter_level(log::LevelFilter::Info)
        .init();
    info!("env_logger inited");

    let addr = "127.0.0.1:8888".to_string();
    let server = Server::tcp_udp_server(
        addr.clone(),
        4,
        Arc::new(message_callback),
        Arc::new(connection_callback),
        Arc::new(datagram_callback),
    );
    server.run();
}
