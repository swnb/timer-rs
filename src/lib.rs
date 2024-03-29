#![warn(missing_docs)]
#![allow(dead_code)]

//! swnb_timer is a timer base on binaryHeap;
//! support async style or callback style;
//! see examples;

mod time;

pub use time::Timer;

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicUsize, Arc};

    use std::sync::atomic::Ordering::SeqCst;
    use std::thread::sleep;
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
    fn set_timeout_multi() {
        let timer = Timer::new();
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone1 = count.clone();
        let _ = timer.set_timeout(
            move || {
                count_clone1.fetch_add(1, SeqCst);
                println!("run callback success");
            },
            Duration::from_secs(1),
        );

        let count_clone2 = count.clone();
        let _ = timer.set_timeout(
            move || {
                count_clone2.fetch_add(1, SeqCst);
                println!("run callback success");
            },
            Duration::from_secs(1),
        );

        std::thread::sleep(Duration::from_secs(1) + Duration::from_millis(20));
        assert_eq!(count.load(SeqCst), 2);
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

    #[test]
    fn test_interval() {
        let timer = Timer::new();

        let count: Arc<AtomicUsize> = Default::default();

        let stop = timer.set_interval(
            {
                let count = count.clone();
                move || {
                    count.fetch_add(1, SeqCst);
                    println!("increase");
                }
            },
            Duration::from_secs(3),
        );

        sleep(Duration::from_secs(7));
        stop();
        sleep(Duration::from_secs(3));
        assert_eq!(count.load(SeqCst), 2);
    }

    #[test]
    fn test_time() {
        let now = std::time::Instant::now();
        let now_after = now + Duration::from_secs(10);
        let now = std::time::Instant::now();
        dbg!(now_after > now);
    }
}
