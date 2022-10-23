use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

use futures::future::{BoxFuture, FutureExt}; // FutureExt is needed for .boxed()
use futures::task::{waker_ref, ArcWake};

struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

impl SharedState {
    fn new() -> Self {
        Self {
            completed: false,
            waker: None,
        }
    }
}

impl TimerFuture {
    pub fn new(count_down: u64) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState::new()));
        let shared_state_cloned = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(count_down));
            let mut shared_state = shared_state_cloned.lock().unwrap();
            shared_state.completed = true;
            if let Some(waker) = shared_state.waker.take() {
                // 这个可以是空的, (当然只是理论上, 起一个线程开销还是很大的)
                // 如果第一次 poll 还没有发生, 或还没有把 waker 放入 shared_state
                // 此时, 显然是不需要也不能够 waker.wake() 的
                // 因为每个 future 至少被 poll 一次,
                // 而当这个 poll 发生时直接会得到 Poll::Ready(()), 就不用 waker 了
                waker.wake();
            }
        }); // 这里不用 join, future 不会完结直到这个线程调了 wake
        Self { shared_state }
    }
}

impl Future for TimerFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            Poll::Ready(())
        } else {
            shared_state.waker = Some(ctx.waker().clone());
            Poll::Pending
        }
    }
}

struct Executor {
    ready_queue: Receiver<Arc<Task>>,
}

struct Spawner {
    task_sender: SyncSender<Arc<Task>>,
}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>, // it's a trait object
    sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // TODO: why self is not ok
        let task = arc_self.clone();
        arc_self.sender.send(task).unwrap();
    }
}

fn channel(n: usize) -> (Spawner, Executor) {
    let (sender, receiver) = sync_channel(n);
    (
        Spawner {
            task_sender: sender,
        },
        Executor {
            ready_queue: receiver,
        },
    )
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let future = future.boxed();
        let sender = self.task_sender.clone();
        let task = Task {
            future: Mutex::new(Some(future)),
            sender,
        };
        self.task_sender.send(Arc::new(task)).unwrap();
    }
}

impl Executor {
    fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let mut ctx = Context::from_waker(&waker);
                dbg!("before polling"); // async' print are after this
                if future.as_mut().poll(&mut ctx).is_pending() {
                    dbg!("polled but pending");
                    // NOTE: future's type is
                    // Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                    // as_mut() convert Pin<Box<...>> to Pin<&mut Self>
                    future_slot.replace(future); // without this, future will never make any progress, so we can not print "world"
                                                 // let _ = std::mem::ManuallyDrop::new(future); // seems no difference with drop(future)
                } else {
                    dbg!("polled and ready");
                    //future_slot.replace(future);
                    //let _ = std::mem::ManuallyDrop::new(future);
                }
            }
        }
    }
}

// #[tokio::main]
// async fn main() {
//     println!("hello");
//     TimerFuture::new(3).await;
//     println!("world");
// }

// fn main() {
//     let (spawner, executor) = channel(10);

//     spawner.spawn(async {
//         println!("hello");
//         TimerFuture::new(3).await;
//         println!("world");
//         TimerFuture::new(3).await;
//         println!("rust");
//     });

//     // spawner.spawn(async {
//     //     TimerFuture::new(2).await;
//     //     thread::sleep(Duration::from_secs(5));
//     //     // println!("hello2");
//     //     // TimerFuture::new(2).await;
//     //     // println!("world2");
//     //     // TimerFuture::new(5).await;
//     //     // println!("rust2");
//     // });

//     drop(spawner); // 只影响 executor.run 能否终止
//     executor.run();
// }

// use futures::stream::StreamExt;
// use futures::stream::
// use futures::channel::mpsc;

// async fn send_recv() {
//     const BUFFER_SIZE: usize = 10;
//     let (mut tx, mut rx) = mpsc::channel::<i32>(BUFFER_SIZE);

//     tx.send(1).await.unwrap();
//     tx.send(2).await.unwrap();
//     drop(tx);

//     // `StreamExt::next` is similar to `Iterator::next`, but returns a
//     // type that implements `Future<Output = Option<T>>`.
//     assert_eq!(Some(1), rx.next().await);
//     assert_eq!(Some(2), rx.next().await);
//     assert_eq!(None, rx.next().await);
// }

//use tokio_stream::{self as stream, StreamExt};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    let (tx1, mut rx1) = oneshot::channel();
    let (tx2, mut rx2) = oneshot::channel();

    tokio::spawn(async move {
        tx1.send("first").unwrap();
    });

    tokio::spawn(async move {
        tx2.send("second").unwrap();
    });

    let mut a = None;
    let mut b = None;

    while a.is_none() || b.is_none() {
        tokio::select! {
            v1 = &mut rx1 => a = Some(v1.unwrap()),
            v2 = &mut rx2 => b = Some(v2.unwrap()),
        }
    }
    // while a.is_none() || b.is_none() {
    //     tokio::select! {
    //         v1 = (&mut rx1), if a.is_none() => a = Some(v1.unwrap()),
    //         v2 = (&mut rx2), if b.is_none() => b = Some(v2.unwrap()),
    //     }
    // }

    dbg!(a);
    dbg!(b);

    //let res = (a.unwrap(), b.unwrap());

    //assert_eq!(res.0, "first");
    //assert_eq!(res.1, "second");
}
