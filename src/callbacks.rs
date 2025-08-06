use std::{net::SocketAddr, sync::Arc, time::Instant};

use log::info;

use crate::{Buffer, SocketRemote, TcpConnection, UdpSocket};

pub type ConnectionCallback = Arc<dyn Fn(Arc<SocketRemote<TcpConnection>>, bool) + Sync + Send>;
pub type MessageCallback =
    Arc<dyn Fn(Arc<SocketRemote<TcpConnection>>, &mut Buffer, Instant) + Sync + Send>;
pub type DatagramCallback =
    Arc<dyn Fn(Arc<SocketRemote<UdpSocket>>, &mut [u8], SocketAddr, Instant) + Sync + Send>;

pub fn default_connection_callback(conn: Arc<SocketRemote<TcpConnection>>, is_connected: bool) {
    info!(
        "connection {} -> {} {}",
        conn.local_addr(),
        conn.peer_addr(),
        if is_connected { "ON" } else { "OFF" }
    );
}
pub fn default_message_callback(
    conn: Arc<SocketRemote<TcpConnection>>,
    buffer: &mut Buffer,
    _receive_time: Instant,
) {
    info!(
        "connection {} -> {} \ncontent:\n{}",
        conn.local_addr(),
        conn.peer_addr(),
        buffer.retrieve_all_as_string()
    );
}
