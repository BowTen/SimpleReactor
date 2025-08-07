use crate::{Reactor, ReactorRemote};

pub struct EventLoopThread<S>
where
    S: crate::ReactorSocket + 'static,
{
    thread: Option<std::thread::JoinHandle<()>>,
    reactor: Option<Reactor<S>>,
    remote: ReactorRemote<S>,
}

impl<S> EventLoopThread<S>
where
    S: crate::ReactorSocket + 'static,
{
    pub fn new(sock_capacity: usize) -> Self {
        let (_, receiver) = crate::reactor_channel::channel();
        let reactor = Reactor::<S>::new(sock_capacity, receiver);
        let reactor_remote = reactor.get_remote();
        EventLoopThread {
            thread: None,
            reactor: Some(reactor),
            remote: reactor_remote,
        }
    }
    pub fn with_reactor(reactor: Reactor<S>) -> Self {
        let reactor_remote = reactor.get_remote();
        EventLoopThread {
            thread: None,
            reactor: Some(reactor),
            remote: reactor_remote,
        }
    }

    pub fn run(&mut self) {
        let reactor = self.reactor.take().unwrap();
        self.thread = Some(std::thread::spawn(move || {
            reactor.run();
        }));
    }

    pub fn get_remote(&self) -> ReactorRemote<S> {
        self.remote.clone()
    }

    pub fn quit(&self) {
        self.remote.quit();
    }

    pub fn wait(mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("Failed to join event loop thread");
        }
    }
}

// impl EventLoopThread<TcpConnection> {
//     pub fn init_with_socket(sock_capacity: usize, socket: TcpConnection) -> (Self, Arc<SocketRemote<TcpConnection>>) {
//         TcpStream::con
//         let (_, receiver) = crate::channel::channel();
//         let mut reactor = Reactor::<TcpConnection>::new(sock_capacity, receiver);
//         reactor.register(socket);
//         let socket_remote = socket.remote().clone();
//         let reactor_remote = reactor.get_remote();
//         (EventLoopThread {
//             thread: None,
//             reactor: Some(reactor),
//             remote: reactor_remote,
//         }, socket_remote)
//     }
// }
