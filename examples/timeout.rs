use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use std::sync::atomic::Ordering::SeqCst;

use swnb_timer::Timer;

fn sleep(duration: Duration) {
    std::thread::sleep(duration);
}

fn main() {
    let timer = Timer::new();
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();
    // count will increase after 1 sec
    let _ = timer.set_timeout(
        move || {
            count_clone.fetch_add(1, SeqCst);
            println!("run callback success");
        },
        Duration::from_secs(1),
    );

    sleep(Duration::from_secs(1) + Duration::from_millis(20));

    assert_eq!(count.load(SeqCst), 1);

    let count_clone = count.clone();
    let cancel_timeout = timer.set_timeout(
        move || {
            count_clone.fetch_add(1, SeqCst);
            println!("run callback success");
        },
        Duration::from_secs(1),
    );

    sleep(Duration::from_millis(20));
    // cancel callback;
    cancel_timeout();
    sleep(Duration::from_secs(1));
    // count still be 1;
    assert_eq!(count.load(SeqCst), 1);
}
