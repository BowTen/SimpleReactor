use std::io::Write;

use log::info;

fn main() {
    env_logger::Builder::from_default_env()
        .init();
    info!("env_logger inited");

    let addr = "127.0.0.1:8888".to_string();
    let mut streams = Vec::new();
    let num_streams = 1;
    for _ in 0..num_streams {
        streams.push(mio::net::TcpStream::connect(addr.parse().unwrap()).unwrap());
    }
    info!("{} connections established", num_streams);
    for (id, stream) in streams.iter_mut().enumerate() {
        let mut msg = format!("Hi, I am TcpClient({})", id);
        unsafe { stream.write_all(msg.as_bytes_mut()).unwrap() };
    }
    info!("{} messages sent", num_streams);
}
