#![warn(missing_docs)]

mod time;

pub use time::Timer;

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicUsize, Arc};

    use std::sync::atomic::Ordering::SeqCst;
    use std::time::Duration;

    use super::*;

    #[test]
    fn set_timeout() {
        let timer = Timer::new();
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let _ = timer.set_timeout(
            move || {
                count_clone.fetch_add(1, SeqCst);
                println!("run callback success");
            },
            Duration::from_secs(1),
        );
        std::thread::sleep(Duration::from_secs(1) + Duration::from_millis(20));
        assert_eq!(count.load(SeqCst), 1);
    }

    #[test]
    fn cancel_timeout() {
        let timer = Timer::new();
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let cancel_timeout = timer.set_timeout(
            move || {
                count_clone.fetch_add(1, SeqCst);
                println!("run callback success");
            },
            Duration::from_secs(1),
        );
        std::thread::sleep(Duration::from_millis(20));
        cancel_timeout();
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(count.load(SeqCst), 0);
    }
}