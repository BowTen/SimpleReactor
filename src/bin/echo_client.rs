use std::sync::Arc;

use log::info;
use simple_reactor::{
    Client, TcpConnection,
    callbacks::{default_connection_callback, default_message_callback},
};
use std::io::{self, Write};

fn main() {
    env_logger::Builder::from_default_env()
        // .filter_level(log::LevelFilter::Info)
        .init();
    info!("env_logger inited");

    let addr = "127.0.0.1:8888".to_string();
    let mut client = Client::<TcpConnection>::new(
        addr,
        Arc::new(default_message_callback),
        Arc::new(default_connection_callback),
    );
    client.listen();

    loop {
        let mut input = String::new();
        print!("请输入要发送的内容: ");
        io::stdout().flush().unwrap();
        if io::stdin().read_line(&mut input).is_ok() {
            if input == "exit\n" {
                break;
            }
            let bytes = input.trim_end().as_bytes();
            client.write(bytes);
        }
    }

    client.shutdown();
}
