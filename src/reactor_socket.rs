pub trait ReactorSocket: Send {
    type Socket: mio::event::Source + ?Sized;
    fn socket(&mut self) -> &mut Self::Socket;
    fn set_interest(&mut self, interest: mio::Interest);
    fn interest(&self) -> mio::Interest;
    fn handle_event(&mut self, event: &mio::event::Event, receive_time: std::time::Instant);
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize>;
    fn stash_output(&mut self, data: &[u8]);
    fn handle_connection(&self, is_connected: bool);
    fn poll_token(&self) -> Option<mio::Token>;
    fn set_poll_token(&mut self, token: mio::Token);
    fn send(&mut self, addr: std::net::SocketAddr, data: &[u8]) -> std::io::Result<usize>;
}
