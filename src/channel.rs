use std::sync::{Arc, Mutex};

pub struct Sender<T>
where
    T: Send,
{
    queue: Arc<Mutex<Vec<T>>>,
}

impl<T> Sender<T>
where
    T: Send,
{
    pub fn send(&self, item: T) {
        let mut queue = self.queue.lock().unwrap();
        queue.push(item);
    }
}

impl<T> Clone for Sender<T>
where
    T: Send,
{
    fn clone(&self) -> Self {
        Sender {
            queue: self.queue.clone(),
        }
    }
}

pub struct Receiver<T>
where
    T: Send,
{
    queue: Arc<Mutex<Vec<T>>>,
}

impl<T> Receiver<T>
where
    T: Send,
{
    pub fn take_all(&self) -> Vec<T> {
        let mut queue = self.queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }

    pub fn get_sender(&self) -> Sender<T> {
        Sender {
            queue: Arc::clone(&self.queue),
        }
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

pub fn channel<T>() -> (Sender<T>, Receiver<T>)
where
    T: Send,
{
    let queue = Arc::new(Mutex::new(Vec::new()));
    (
        Sender {
            queue: queue.clone(),
        },
        Receiver { queue: queue },
    )
}
