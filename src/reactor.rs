use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::ThreadId,
    usize,
};

use log::{error, info, trace, warn};
use mio::{Events, Poll, Token, Waker};
use slab::Slab;

use crate::{
    ReactorRemote,
    reactor_channel::{Receiver, Sender},
};

pub fn u64_current_thread_id() -> u64 {
    unsafe { std::mem::transmute::<ThreadId, u64>(std::thread::current().id()) }
}

pub enum ReactorSignal<S>
where
    S: crate::ReactorSocket,
{
    Quit,
    Register(S),
    ShutDown(Token),
    ReRegister(Token, mio::Interest),
    Write(Token, Vec<u8>),
    Send(Token, SocketAddr, Vec<u8>), // For UDP sockets
}

impl<S> ReactorSignal<S>
where
    S: crate::ReactorSocket,
{
    pub fn type_str(&self) -> &str {
        match self {
            Self::Quit => "Quit",
            Self::Register(_) => "Register",
            Self::ShutDown(_) => "ShutDown",
            Self::ReRegister(_, _) => "ReRegister",
            Self::Write(_, _) => "Write",
            Self::Send(_, _, _) => "DatagramSend",
        }
    }
}

pub struct Reactor<S>
where
    S: crate::ReactorSocket,
{
    poll: Poll,
    events: mio::Events,
    sockets: Slab<S>,
    signal_receiver: Receiver<ReactorSignal<S>>,
    quit: bool,
    waker: Arc<Waker>,
    thread_id: Arc<AtomicU64>,
}

impl<S> Reactor<S>
where
    S: crate::ReactorSocket,
{
    pub fn new(sock_capacity: usize) -> Self {
        let poll = Poll::new().unwrap();
        let waker = Arc::new(Waker::new(poll.registry(), Token(usize::MAX)).unwrap());
        Reactor {
            poll,
            events: Events::with_capacity(1024),
            sockets: Slab::with_capacity(sock_capacity),
            signal_receiver: Receiver::new(Arc::new(Mutex::new(Vec::new()))),
            quit: false,
            waker,
            thread_id: Arc::new(AtomicU64::new(u64::MAX / 2)),
        }
    }

    pub fn get_remote(&self) -> ReactorRemote<S> {
        ReactorRemote::new(self.get_sender())
    }

    pub fn get_sender(&self) -> Sender<ReactorSignal<S>> {
        Sender::new(
            self.signal_receiver.queue.clone(),
            self.waker.clone(),
            self.thread_id.clone(),
        )
    }

    pub fn run(mut self) {
        self.thread_id
            .store(u64_current_thread_id(), Ordering::Relaxed);
        // 运行事件循环
        while !self.quit {
            self.poll
                .poll(&mut self.events, None)
                .expect("Failed to poll events");
            let receive_time = std::time::Instant::now();

            for event in self.events.iter() {
                trace!("reveice event with token({})", event.token().0);
                if let Some(socket) = self.sockets.get_mut(event.token().0) {
                    socket.handle_event(event, receive_time);
                }
            }

            let signals = self.signal_receiver.take_all();
            for signal in signals {
                self.handle_signal(signal);
            }
        }
        info!("Reactor has quit");
    }

    fn handle_signal(&mut self, signal: ReactorSignal<S>) {
        trace!("handle signal: {}", signal.type_str());
        match signal {
            ReactorSignal::Quit => self.quit(),
            ReactorSignal::Register(socket) => {
                self.register(socket);
                ()
            }
            ReactorSignal::ShutDown(token) => self.shutdown(token),
            ReactorSignal::ReRegister(token, interest) => self.reregister(token, interest),
            ReactorSignal::Write(token, data) => self.write(token, data),
            ReactorSignal::Send(token, addr, data) => self.send(token, addr, data),
        }
    }

    fn quit(&mut self) {
        info!("Reactor is quitting");
        self.quit = true;
    }

    pub fn register(&mut self, socket: S) -> Option<Token> {
        let interest = socket.interest();
        let token = Token(self.sockets.insert(socket));
        if self
            .poll
            .registry()
            .register(self.sockets[token.0].socket(), token, interest)
            .is_err()
        {
            self.sockets.remove(token.0);
            error!("Failed to register socket");
            return None;
        }
        self.sockets[token.0].set_poll_token(token);
        self.sockets[token.0].handle_establish(true);
        Some(token)
    }

    fn shutdown(&mut self, token: Token) {
        if !self.sockets.contains(token.0) {
            error!("no such token to shutdown: Token({})", token.0);
            return;
        }
        if self
            .poll
            .registry()
            .deregister(self.sockets[token.0].socket())
            .is_err()
        {
            error!("Failed to deregister socket");
        }
        self.sockets[token.0].handle_establish(false);
        self.sockets.remove(token.0);
    }

    fn reregister(&mut self, token: Token, interest: mio::Interest) {
        let socket = &mut self.sockets[token.0];
        socket.set_interest(interest);
        if self
            .poll
            .registry()
            .reregister(socket.socket(), token, interest)
            .is_err()
        {
            error!("Failed to reregister socket");
        }
    }

    fn write(&mut self, token: Token, data: Vec<u8>) {
        if let Some(socket) = self.sockets.get_mut(token.0) {
            if !socket.interest().is_writable() {
                let mut total_written = 0;
                loop {
                    match socket.write(data[total_written..].as_ref()) {
                        Ok(bytes_written) if bytes_written == 0 => {
                            error!(
                                "Connection closed while writing to socket with token {:?}",
                                token
                            );
                            self.shutdown(token);
                            return;
                        }
                        Ok(bytes_written) => {
                            total_written += bytes_written;
                            if total_written == data.len() {
                                break;
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            trace!("Socket would block on write");
                            break;
                        }
                        Err(e) => {
                            error!("Failed to write to socket: {}", e);
                            self.shutdown(token);
                            return;
                        }
                    }
                }
                if total_written < data.len() {
                    socket.stash_output(data[total_written..].as_ref());
                    let interest = socket.interest().add(mio::Interest::WRITABLE);
                    self.reregister(token, interest);
                }
            } else {
                socket.stash_output(data[..].as_ref());
            }
        } else {
            error!("Socket with token {:?} not found", token);
        }
    }

    // Only call on UdpSocket
    fn send(&mut self, token: Token, addr: SocketAddr, data: Vec<u8>) {
        if let Some(socket) = self.sockets.get_mut(token.0) {
            match socket.send(addr, data.as_ref()) {
                Ok(bytes_sent) => {
                    trace!("Sent {} bytes to {}", bytes_sent, addr);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    warn!("Socket would block on send, drop data!");
                }
                Err(e) => {
                    error!("Failed to send data: {}", e);
                    // TODO: solve some error to shutdown
                    // self.shutdown(token);
                }
            }
        } else {
            error!("Socket with token {:?} not found", token);
        }
    }
}
