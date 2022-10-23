use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, t: T) {
        self.shared.inner.lock().unwrap().queue.push_back(t);
        self.shared.available.notify_one();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders += 1;
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let senders = {
            let mut inner = self.shared.inner.lock().unwrap();
            inner.senders -= 1;
            inner.senders
        };
        if senders == 0 {
            self.shared.available.notify_one();
        }
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Option<T> {
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            match inner.queue.pop_front() {
                Some(t) => return Some(t),
                None if inner.senders == 0 => return None,
                None => {
                    inner = self.shared.available.wait(inner).unwrap();
                }
            }
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.available.notify_all();
    }
}

struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,
}

struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize, // count of senders and receiver
}

impl<T> Shared<T> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                queue: Default::default(),
                senders: 1,
            }),
            available: Condvar::new(),
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared::new());
    (
        Sender {
            shared: Arc::clone(&shared),
        },
        Receiver { shared },
    )
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_pong() {
        let (tx, rx) = channel();
        tx.send(42);
        assert_eq!(rx.recv(), Some(42));
    }

    #[test]
    fn test_drop_last_sender() {
        let (tx, rx) = channel::<i32>();
        let _ = tx;
        assert_eq!(rx.recv(), None);
    }
}
