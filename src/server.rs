use log::warn;

use crate::{
    Acceptor, EventLoopThreadPool, Reactor, ReactorRemote, TcpConnection,
    callbacks::{ConnectionCallback, MessageCallback},
    reactor::ReactorSignal,
};

pub struct Server {
    addr: String,
    main_reactor: Reactor<Acceptor>,
    main_reactor_remote: ReactorRemote<Acceptor>,
    // udp_event_loop_thread: Option<EventLoopThread<UdpConnection>>,
    event_loop_thread_pool: Option<EventLoopThreadPool<TcpConnection>>,
    connection_callback: ConnectionCallback,
    message_callback: MessageCallback,
}

impl Server {
    pub fn new(
        addr: String,
        mut num_threads: usize,
        message_callback: MessageCallback,
        connection_callback: ConnectionCallback,
    ) -> Self {
        if num_threads == 0 {
            warn!("Number of threads is 0, using 1 instead");
            num_threads = 1;
        }
        let (_, receiver) = crate::channel::channel::<ReactorSignal<Acceptor>>();
        let reactor = Reactor::new(2, receiver);
        let remote = reactor.get_remote();
        Server {
            addr,
            main_reactor: reactor,
            main_reactor_remote: remote,
            event_loop_thread_pool: Some(EventLoopThreadPool::new(num_threads)),
            connection_callback,
            message_callback,
        }
    }

    pub fn run(mut self) {
        self.event_loop_thread_pool.as_mut().unwrap().run();
        let acceptor = Acceptor::new(
            self.addr.clone(),
            self.event_loop_thread_pool.take().unwrap(),
            self.connection_callback.clone(),
            self.message_callback.clone(),
        );
        self.main_reactor.register(acceptor);
        println!("Server is running on {}", self.addr);
        self.main_reactor.run();
    }

    pub fn get_quiter(&self) -> ServerQuiter {
        let acceptor_remote = self.main_reactor_remote.clone();
        let tcp_remotes = self.event_loop_thread_pool.as_ref().unwrap().get_remotes();
        ServerQuiter::new(acceptor_remote, tcp_remotes)
    }
}

pub struct ServerQuiter {
    acceptor_remote: ReactorRemote<Acceptor>,
    tcp_remotes: Vec<ReactorRemote<TcpConnection>>,
}

impl ServerQuiter {
    pub fn new(
        acceptor_remote: ReactorRemote<Acceptor>,
        tcp_remotes: Vec<ReactorRemote<TcpConnection>>,
    ) -> Self {
        ServerQuiter {
            acceptor_remote,
            tcp_remotes,
        }
    }

    pub fn quit(&self) {
        self.acceptor_remote.quit();
        for remote in &self.tcp_remotes {
            remote.quit();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        thread::{self, sleep},
        time::Duration,
    };

    use log::info;

    use crate::{Server, TcpConnection};

    fn message_callback(
        remote: Arc<crate::SocketRemote<TcpConnection>>,
        buffer: &mut crate::Buffer,
        receive_time: std::time::Instant,
    ) {
        info!(
            "{:?}\nReceived message from {}: {:?}",
            receive_time,
            remote.local_addr(),
            buffer.retrieve_all_as_string()
        );
    }

    fn connection_callback(remote: Arc<crate::SocketRemote<TcpConnection>>, is_connected: bool) {
        if is_connected {
            info!("New connection established: {}", remote.local_addr());
        } else {
            info!("Connection closed: {}", remote.local_addr());
        }
    }

    #[test]
    fn run_one_sec_and_quit() {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
        info!("env_logger inited");

        let addr = "127.0.0.1:8888".to_string();
        let server = Server::new(
            addr.clone(),
            1,
            Arc::new(message_callback),
            Arc::new(connection_callback),
        );
        let quiter = server.get_quiter();

        let tid = thread::spawn(move || {
            sleep(Duration::from_millis(1000));
            quiter.quit();
        });
        server.run();

        tid.join().expect("fail to wait thread");
    }
}
