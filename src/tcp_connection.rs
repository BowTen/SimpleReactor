use std::{io::Write, sync::{atomic::AtomicBool, Arc}};

use log::{error, trace, warn};
use mio::{Interest, Waker, net::TcpStream};

use crate::{
    Buffer, ReactorSocket, SocketRemote,
    callbacks::{ConnectionCallback, MessageCallback},
    reactor_channel::Sender,
    reactor::ReactorSignal,
};

pub struct TcpConnection {
    stream: TcpStream,
    connection_callback: ConnectionCallback,
    message_callback: MessageCallback,
    input_buffer: Buffer,
    output_buffer: Buffer,
    signal_sender: Sender<ReactorSignal<TcpConnection>>,
    waker: Arc<Waker>,
    remote: Option<Arc<SocketRemote<TcpConnection>>>,
    interest: mio::Interest,
    poll_token: Option<mio::Token>,
    pub is_established: Arc<AtomicBool>,
}

impl TcpConnection {
    pub fn new(
        stream: TcpStream,
        connection_callback: ConnectionCallback,
        message_callback: MessageCallback,
        interest: mio::Interest,
        signal_sender: Sender<ReactorSignal<TcpConnection>>,
        waker: Arc<Waker>,
    ) -> Self {
        TcpConnection {
            stream,
            connection_callback,
            message_callback,
            input_buffer: Buffer::new(),
            output_buffer: Buffer::new(),
            signal_sender,
            waker,
            remote: None,
            interest,
            poll_token: None,
            is_established: Arc::new(AtomicBool::new(false)),
        }
    }

    // must call after register
    pub fn remote(&self) -> &Arc<SocketRemote<TcpConnection>> {
        self.remote
            .as_ref()
            .expect("must call register before accessing remote")
    }

    fn handle_read(&mut self, receive_time: std::time::Instant) {
        let mut total_read = 0;
        loop {
            match self.input_buffer.read_tcp_stream(&mut self.stream) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        trace!(
                            "Connection closed by peer: {}",
                            self.stream.peer_addr().unwrap()
                        );
                        if total_read > 0 {
                            (self.message_callback)(
                                self.remote().clone(),
                                &mut self.input_buffer,
                                receive_time,
                            );
                        }
                        self.remote().shutdown();
                        return;
                    }
                    total_read += bytes_read;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    warn!("Failed to read from stream: {}, shutdown it", e);
                    self.remote().shutdown();
                    return;
                }
            }
        }
        trace!("Total bytes read: {}", total_read);
        if total_read > 0 {
            (self.message_callback)(self.remote().clone(), &mut self.input_buffer, receive_time);
        }
    }
    fn handle_write(&mut self) {
        let data = self.output_buffer.as_slice();
        let mut total_written = 0;
        if total_written < data.len() {
            loop {
                match self.stream.write(data[total_written..].as_ref()) {
                    Ok(bytes_written) if bytes_written == 0 => {
                        error!("Connection closed while writing to socket");
                        self.remote().shutdown();
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
                        self.remote().shutdown();
                        return;
                    }
                }
            }
        }
        self.output_buffer.retrieve(total_written);
        if total_written > 0
            && self.output_buffer.readable_bytes() == 0
            && self.interest.is_writable()
        {
            trace!("No more data to write, removing writable interest");
            self.remote().reregister(
                self.interest
                    .remove(mio::Interest::WRITABLE)
                    .unwrap_or(Interest::READABLE),
            );
        }
    }
}

impl ReactorSocket for TcpConnection {
    type Socket = TcpStream;

    fn handle_event(&mut self, event: &mio::event::Event, receive_time: std::time::Instant) {
        if event.is_readable() {
            self.handle_read(receive_time);
        }
        if self.interest.is_writable() && event.is_writable() {
            self.handle_write();
        }
    }

    fn socket(&mut self) -> &mut Self::Socket {
        &mut self.stream
    }

    fn interest(&self) -> mio::Interest {
        self.interest
    }

    fn set_interest(&mut self, interest: mio::Interest) {
        self.interest = interest;
    }

    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.stream.write(data)
    }

    fn stash_output(&mut self, data: &[u8]) {
        self.output_buffer.append(data);
    }

    fn handle_establish(&self, is_established: bool) {
        self.is_established
            .store(is_established, std::sync::atomic::Ordering::Relaxed);
        (self.connection_callback)(self.remote().clone(), is_established);
    }

    fn poll_token(&self) -> Option<mio::Token> {
        self.poll_token
    }

    fn set_poll_token(&mut self, token: mio::Token) {
        self.poll_token = Some(token);
        self.remote = Some(Arc::new(SocketRemote::new(
            self.stream.local_addr().unwrap(),
            self.stream.peer_addr().unwrap(),
            token,
            self.signal_sender.clone(),
            self.waker.clone(),
            self.is_established.clone(),
        )));
    }

    fn send(&mut self, _addr: std::net::SocketAddr, _data: &[u8]) -> std::io::Result<usize> {
        // TCP does not support send to specific address
        panic!("TCP connection does not support send to specific address");
    }

    fn is_established(&self) -> bool {
        self.is_established.load(std::sync::atomic::Ordering::Relaxed)
    }
}
