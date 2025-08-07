use std::{sync::Arc, time::Instant};

use log::info;
use simple_reactor::{
    Buffer, Client, SocketRemote, TcpConnection, callbacks::default_connection_callback,
};
use std::io::{self, Write};

fn message_callback(
    _conn: Arc<SocketRemote<TcpConnection>>,
    buffer: &mut Buffer,
    _receive_time: Instant,
) {
    let content = buffer.retrieve_all_as_string();
    println!("server: {}", content);
}

fn main() {
    env_logger::Builder::from_default_env()
        // .filter_level(log::LevelFilter::Info)
        .init();
    info!("env_logger inited");

    let addr = "127.0.0.1:8888".to_string();
    let mut client = Client::<TcpConnection>::new(
        addr,
        Arc::new(message_callback),
        Arc::new(default_connection_callback),
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
            client.write(bytes);
        }
    }

    client.shutdown();
}
