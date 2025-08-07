use log::info;

fn main() {
    env_logger::Builder::from_default_env().init();
    info!("env_logger inited");

    let addr = "127.0.0.1:8888".to_string();
    let socket = mio::net::UdpSocket::bind("0.0.0.0:0".parse().unwrap()).unwrap();
    let msg = "Hello, UDP server!";
    let mut buf = msg.as_bytes().to_owned();
    if socket.send_to(&buf, addr.clone().parse().unwrap()).is_ok() {
        info!("Sent message to {}", addr);
    } else {
        info!("Failed to send message to {}", addr);
    }
}
