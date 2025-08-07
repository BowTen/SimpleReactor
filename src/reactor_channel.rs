use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use log::error;
use mio::Waker;

use crate::reactor::u64_current_thread_id;

pub struct Sender<T>
where
    T: Send,
{
    queue: Arc<Mutex<Vec<T>>>,
    waker: Arc<Waker>,
    thread_id: Arc<AtomicU64>,
}

impl<T> Sender<T>
where
    T: Send,
{
    pub fn new(queue: Arc<Mutex<Vec<T>>>, waker: Arc<Waker>, thread_id: Arc<AtomicU64>) -> Self {
        Self {
            queue,
            waker,
            thread_id,
        }
    }

    pub fn send(&self, item: T) {
        {
            let mut queue = self.queue.lock().unwrap();
            queue.push(item);
        }
        if self.thread_id.load(Ordering::Relaxed) != u64_current_thread_id() {
            if self.waker.wake().is_err() {
                error!("Failed to wake reactor up!")
            }
        }
    }
}

impl<T> Clone for Sender<T>
where
    T: Send,
{
    fn clone(&self) -> Self {
        Sender {
            queue: self.queue.clone(),
            waker: Arc::clone(&self.waker),
            thread_id: Arc::clone(&self.thread_id),
        }
    }
}

pub struct Receiver<T>
where
    T: Send,
{
    pub queue: Arc<Mutex<Vec<T>>>,
}

impl<T> Receiver<T>
where
    T: Send,
{
    pub fn new(queue: Arc<Mutex<Vec<T>>>) -> Self {
        Self { queue }
    }

    pub fn take_all(&self) -> Vec<T> {
        let mut queue = self.queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }
}

impl<T> Clone for Receiver<T>
where
    T: Send,
{
    fn clone(&self) -> Self {
        Receiver {
            queue: self.queue.clone(),
        }
    }
}
