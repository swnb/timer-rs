use std::{
    sync::{atomic::AtomicUsize, Arc},
    thread::sleep,
    time::Duration,
};

use std::sync::atomic::Ordering::SeqCst;

use swnb_timer::Timer;

fn main() {
    let timer = Timer::new();
    let count: Arc<AtomicUsize> = Default::default();
    let count_clone = count.clone();
    // count will increase every 1 sec
    let _ = timer.set_interval(
        move || {
            count_clone.fetch_add(1, SeqCst);
            println!("run callback success");
        },
        Duration::from_secs(2),
    );

    sleep(Duration::from_secs(5));

    assert_eq!(count.load(SeqCst), 2);
}
