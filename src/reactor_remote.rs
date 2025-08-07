use log::trace;

use crate::{ReactorSocket, reactor::ReactorSignal, reactor_channel::Sender};

pub struct ReactorRemote<S>
where
    S: ReactorSocket,
{
    sender: Sender<ReactorSignal<S>>,
}

impl<S> Clone for ReactorRemote<S>
where
    S: ReactorSocket,
{
    fn clone(&self) -> Self {
        ReactorRemote {
            sender: self.sender.clone(),
        }
    }
}

impl<S> ReactorRemote<S>
where
    S: ReactorSocket,
{
    pub fn new(sender: Sender<ReactorSignal<S>>) -> Self {
        ReactorRemote { sender }
    }

    pub fn get_sender(&self) -> Sender<ReactorSignal<S>> {
        self.sender.clone()
    }

    pub fn register(&self, socket: S) {
        self.sender.send(ReactorSignal::Register(socket));
    }

    pub fn quit(&self) {
        trace!("Sending quit signal to reactor");
        self.sender.send(ReactorSignal::Quit);
    }
}
