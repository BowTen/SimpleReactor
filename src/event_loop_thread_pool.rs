use crate::{EventLoopThread, ReactorRemote};

pub struct EventLoopThreadPool<S>
where
    S: crate::ReactorSocket + 'static,
{
    reactor_index: usize,
    threads: Vec<EventLoopThread<S>>,
}

impl<S> EventLoopThreadPool<S>
where
    S: crate::ReactorSocket + 'static,
{
    pub fn new(thread_count: usize) -> Self {
        let mut threads = Vec::with_capacity(thread_count);
        for _ in 0..thread_count {
            threads.push(EventLoopThread::new(1024));
        }
        EventLoopThreadPool {
            threads,
            reactor_index: 0,
        }
    }

    pub fn reactor_index(&self) -> usize {
        self.reactor_index
    }

    pub fn run(&mut self) {
        for thread in &mut self.threads {
            thread.run();
        }
    }

    pub fn get_next_reactor(&mut self) -> ReactorRemote<S> {
        let remote = self.threads[self.reactor_index].get_remote();
        self.reactor_index = (self.reactor_index + 1) % self.threads.len();
        remote
    }

    pub fn get_remotes(&self) -> Vec<ReactorRemote<S>> {
        self.threads
            .iter()
            .map(|thread| thread.get_remote())
            .collect()
    }

    pub fn quit(&self) {
        for thread in &self.threads {
            thread.quit();
        }
    }

    pub fn wait(self) {
        for thread in self.threads {
            thread.wait();
        }
    }
}
