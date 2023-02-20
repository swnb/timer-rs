use std::alloc::System;
use std::cmp::{Eq, Ord, Ordering};
use std::collections::BinaryHeap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc, Condvar, Mutex,
};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

enum Callback {
    Once(Option<Box<dyn FnOnce() + Send>>),
    Mut(Box<dyn FnMut() + Send>),
}

// scheduler unit
struct Task {
    time: std::time::Instant,
    is_deleted: Arc<AtomicBool>,
    reuseable: bool,
    callback: Callback,
    duration: Duration,
}

impl Task {
    fn call(&mut self) {
        match self.callback {
            Callback::Once(ref mut f) => {
                if let Some(f) = f.take() {
                    f()
                }
            }
            Callback::Mut(ref mut f) => f(),
        }
    }

    fn is_deleted(&self) -> bool {
        self.is_deleted.load(SeqCst)
    }
}

impl PartialOrd for Task {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.time.cmp(&self.time))
    }
}

impl PartialEq for Task {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Ord for Task {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        other.time.cmp(&self.time)
    }
}

impl Eq for Task {}

/// Timer store all timeout callback base on binaryHeap;
/// The callback function will be triggered when the time expires
///
/// Example
///
/// ```
/// use swnb_timer::Timer;
/// use std::time::Duration;
///
/// let timer = Timer::new();
///
/// timer.set_timeout(||{
///     println!("after 1 sec");
/// },Duration::from_secs(1));
///
/// std::thread::sleep(Duration::from_secs(2));
/// ```
pub struct Timer {
    thread_handler: std::thread::JoinHandle<()>,
    cond: Arc<(Condvar, Mutex<BinaryHeap<Task>>)>,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    /// create new Timer, this method will create one thread to handle all task base on binaryHeap;
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    /// use swnb_timer::Timer;
    /// use std::time::Duration;
    ///
    /// let timer = Timer::new();
    ///
    /// timer.set_timeout(||{
    ///     println!("after 1 sec");
    /// },Duration::from_secs(1));
    ///
    /// timer.set_timeout(||{
    ///     println!("after 2 sec");
    /// },Duration::from_secs(2));
    ///
    /// std::thread::sleep(Duration::from_secs(3));
    ///
    /// ```
    ///
    /// Async usage:
    /// ```
    /// use swnb_timer::Timer;
    /// use std::time::Duration;
    ///
    /// let timer = Timer::new();
    ///
    /// let async_block = async {
    ///     timer.wait(Duration::from_secs(1)).await;
    ///     println!("after 1 sec");
    /// };
    /// // blocking_on(async_block);
    /// ```
    ///
    pub fn new() -> Self {
        let heap = BinaryHeap::new();
        let cond = Arc::new((Condvar::new(), Mutex::new(heap)));
        let thread_handler = Timer::handle_task(cond.clone());
        Timer {
            thread_handler,
            cond,
        }
    }

    fn handle_task(cond: Arc<(Condvar, Mutex<BinaryHeap<Task>>)>) -> std::thread::JoinHandle<()> {
        let worker = move || {
            let mut locker = cond.1.lock().unwrap();
            loop {
                loop {
                    match locker.peek() {
                        Some(&Task {
                            time,
                            ref is_deleted,
                            ..
                        }) => {
                            if is_deleted.load(SeqCst) {
                                locker.pop();
                            } else {
                                let now = std::time::Instant::now();
                                if time <= now {
                                    break;
                                } else {
                                    let (new_locker, _) = cond
                                        .0
                                        .wait_timeout(locker, time.duration_since(now))
                                        .unwrap();
                                    locker = new_locker;
                                }
                            }
                        }
                        None => {
                            locker = cond.0.wait(locker).unwrap();
                        }
                    }
                }

                while let Some(task) = locker.peek() {
                    if task.is_deleted() {
                        locker.pop();
                        continue;
                    }
                    let now = Instant::now();

                    if task.time <= now {
                        let mut task = locker.pop().unwrap();
                        task.call();
                        if task.reuseable {
                            task.time = now + task.duration;
                            locker.push(task);
                        }
                    } else {
                        break;
                    }
                }
            }
        };

        std::thread::Builder::new()
            .name("swnb-timer".into())
            .spawn(worker)
            .unwrap()
    }

    /// set_timeout accept two arguments, callback and duration;
    /// callback will run after duration;
    /// if you want to cancel callback before the deadline,
    /// set_timeout return cancel function,
    /// run it will cancel current timeout callback;
    ///
    /// # Examples
    ///
    /// set_timeout:
    ///
    /// ```
    /// use swnb_timer::Timer;
    /// use std::time::Duration;
    ///
    /// let timer = Timer::new();
    ///
    /// timer.set_timeout(||{
    ///     println!("after 1 sec");
    /// },Duration::from_secs(1));
    ///
    /// timer.set_timeout(||{
    ///     println!("after 2 sec");
    /// },Duration::from_secs(2));
    ///
    /// std::thread::sleep(Duration::from_secs(3));
    /// ```
    ///
    /// cancel_callback:
    ///
    /// ```
    /// use swnb_timer::Timer;
    /// use std::time::Duration;
    ///
    /// let timer = Timer::new();
    ///
    /// let cancel = timer.set_timeout(||{
    ///     println!("after 2 sec");
    /// },Duration::from_secs(2));
    ///
    /// timer.set_timeout(move ||{
    ///    cancel();
    ///    println!("cancel previous timeout callback");
    /// },Duration::from_secs(1));
    ///
    /// std::thread::sleep(Duration::from_secs(3));
    /// ```
    pub fn set_timeout(
        &self,
        callback: impl FnOnce() + 'static + Send,
        duration: std::time::Duration,
    ) -> impl FnOnce() + Sync + 'static {
        let now = std::time::Instant::now();
        let is_deleted = Arc::new(AtomicBool::new(false));

        let task = Task {
            callback: Callback::Once(Some(Box::new(callback))),
            is_deleted: is_deleted.clone(),
            time: now + duration,
            reuseable: false,
            duration,
        };

        self.push_task(task);

        move || is_deleted.store(true, SeqCst)
    }

    /// set_interval is basically consistent with set_timeout
    /// callback will run every duration;
    /// if you want to cancel interval,
    /// set_interval return cancel function,
    /// run it will cancel current interval callback;
    ///
    /// # Examples
    ///
    /// set_interval:
    ///
    /// ```
    /// use swnb_timer::Timer;
    /// use std::time::Duration;
    ///
    /// let timer = Timer::new();
    ///
    /// timer.set_interval(||{
    ///     println!("every 1 sec");
    /// },Duration::from_secs(1));
    ///
    /// timer.set_interval(||{
    ///     println!("every 2 sec");
    /// },Duration::from_secs(2));
    ///
    /// std::thread::sleep(Duration::from_secs(3));
    /// ```
    ///
    /// cancel_interval:
    ///
    /// ```
    /// use swnb_timer::Timer;
    /// use std::time::Duration;
    ///
    /// let timer = Timer::new();
    ///
    /// let cancel = timer.set_interval(||{
    ///     println!("every 2 sec");
    /// },Duration::from_secs(2));
    ///
    /// timer.set_timeout(move ||{
    ///    cancel();
    ///    println!("cancel previous timeout callback");
    /// },Duration::from_secs(1));
    ///
    /// std::thread::sleep(Duration::from_secs(7));
    /// ```
    pub fn set_interval(
        &self,
        callback: impl FnMut() + 'static + Send,
        duration: std::time::Duration,
    ) -> impl FnOnce() + Sync + 'static {
        let now = std::time::Instant::now();
        let is_deleted: Arc<AtomicBool> = Default::default();

        let task = Task {
            callback: Callback::Mut(Box::new(callback)),
            is_deleted: is_deleted.clone(),
            time: now + duration,
            reuseable: true,
            duration,
        };

        self.push_task(task);

        move || is_deleted.store(true, SeqCst)
    }

    fn push_task(&self, task: Task) {
        let mut locker = self.cond.1.lock().unwrap();
        locker.push(task);
        drop(locker);
        self.cond.0.notify_one();
    }

    /// wait for `duration` time
    ///
    /// Examples
    ///
    /// ```
    /// use swnb_timer::Timer;
    /// use std::time::Duration;
    ///
    /// let timer = Timer::new();
    ///
    /// let async_block = async {
    ///     timer.wait(Duration::from_secs(1)).await;
    /// };
    ///
    /// // blocking_on(async_block);
    /// ```
    ///
    pub async fn wait(&self, duration: std::time::Duration) {
        let future_timer = FutureTimer::new(duration, self);
        future_timer.await
    }
}

// FutureTimer impl Future
// for Timer to do async wait
struct FutureTimer<'a> {
    duration: std::time::Duration,
    is_set_timeout: AtomicBool,
    is_resolved: Arc<AtomicBool>,
    timer: &'a Timer,
}

impl<'a> Future for FutureTimer<'a> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = self
            .is_set_timeout
            .compare_exchange(false, true, SeqCst, SeqCst);

        if result.is_ok() {
            let is_resolved = self.is_resolved.clone();
            let waker = cx.waker().clone();
            let _ = self.timer.set_timeout(
                move || {
                    is_resolved.store(true, SeqCst);
                    waker.wake();
                },
                self.duration,
            );
            Poll::Pending
        } else if self.is_resolved.load(SeqCst) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl<'a> FutureTimer<'a> {
    fn new<'b: 'a>(duration: std::time::Duration, timer: &'b Timer) -> Self {
        FutureTimer {
            duration,
            is_set_timeout: AtomicBool::new(false),
            is_resolved: Arc::new(AtomicBool::new(false)),
            timer,
        }
    }
}

mod test {
    use std::{default, mem::size_of, time::Instant};

    #[test]
    fn test_size() {
        use super::Task;
        dbg!(size_of::<Task>());
        dbg!(size_of::<Instant>());
        let v: bool = Default::default();
        dbg!(v);
    }
}
