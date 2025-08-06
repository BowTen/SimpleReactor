use log::info;
use simple_reactor::{Buffer, Server, SocketRemote, TcpConnection};
use std::sync::Arc;

fn message_callback(
    remote: Arc<SocketRemote<TcpConnection>>,
    buffer: &mut Buffer,
    _receive_time: std::time::Instant,
) {
    info!(
        "Received message from {}: {:?}",
        remote.local_addr(),
        buffer.retrieve_all_as_string()
    );
}

fn connection_callback(remote: Arc<SocketRemote<TcpConnection>>, is_connected: bool) {
    if is_connected {
        info!("New connection established: {}", remote.peer_addr());
    } else {
        info!("Connection closed: {}", remote.peer_addr());
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        // .filter_level(log::LevelFilter::Info)
        .init();
    info!("env_logger inited");

    let addr = "127.0.0.1:8888".to_string();
    let server = Server::new(
        addr.clone(),
        4,
        Arc::new(message_callback),
        Arc::new(connection_callback),
    );
    server.run();
}
