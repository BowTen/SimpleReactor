use std::{io::Write, thread::sleep, time::Duration};

use log::info;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("env_logger inited");

    let addr = "127.0.0.1:8888".to_string();
    let keep_secs = 10;
    let num_per_sec = 100;
    let mut index = 0;
    let mut streams = Vec::new();
    streams.resize_with(keep_secs, || Vec::new());

    loop {
        streams[index] = Vec::new();
        for _ in 0..num_per_sec {
            streams[index].push(mio::net::TcpStream::connect(addr.parse().unwrap()).unwrap());
        }
        // info!("push {} stream to streams[{}]", num_per_sec, index);
        index = (index + 1) % keep_secs;

        let mut total_send = 0;
        for (i, v) in streams.iter_mut().enumerate() {
            for (j, stream) in v.iter_mut().enumerate() {
                let msg = format!("Hi, I am stream[{}][{}]", i, j);
                let mut buf = msg.as_bytes().to_owned();
                if stream.write(&mut buf).is_ok() {
                    total_send += 1;
                }
            }
        }
        info!("send {} messages to {}", total_send, addr);

        sleep(Duration::from_secs(1));
    }
}
