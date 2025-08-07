use std::sync::Arc;

use log::trace;
use mio::Waker;

use crate::{ReactorSocket, reactor_channel::Sender, reactor::ReactorSignal};

pub struct ReactorRemote<S>
where
    S: ReactorSocket,
{
    sender: Sender<ReactorSignal<S>>,
    waker: Arc<Waker>,
}

impl<S> Clone for ReactorRemote<S>
where
    S: ReactorSocket,
{
    fn clone(&self) -> Self {
        ReactorRemote {
            sender: self.sender.clone(),
            waker: Arc::clone(&self.waker),
        }
    }
}

impl<S> ReactorRemote<S>
where
    S: ReactorSocket,
{
    pub fn new(sender: Sender<ReactorSignal<S>>, waker: Arc<Waker>) -> Self {
        ReactorRemote { sender, waker }
    }

    pub fn get_sender(&self) -> Sender<ReactorSignal<S>> {
        self.sender.clone()
    }

    pub fn get_waker(&self) -> Arc<Waker> {
        Arc::clone(&self.waker)
    }

    pub fn register(&self, socket: S) {
        self.sender.send(ReactorSignal::Register(socket));
        self.waker.wake().expect("Failed to wake reactor");
    }

    pub fn quit(&self) {
        trace!("Sending quit signal to reactor");
        self.sender.send(ReactorSignal::Quit);
        self.waker.wake().expect("Failed to wake reactor");
    }
}
