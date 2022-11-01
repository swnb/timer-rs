# timer-rs

> timer can execute a function, after waiting a specified number of time. also support async style api

## install

```bash
  cargo add swnb-timer
```

## set timeout callback

```rust
use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use std::sync::atomic::Ordering::SeqCst;

use swnb_timer::Timer;

fn main(){
    let timer = Timer::new();
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();
    let duration = Duration::from_secs(1);
    // count will increase after 1 sec
    let _ = timer.set_timeout(
        move || {
            count_clone.fetch_add(1, SeqCst);
            println!("run callback success");
        },
        duration,
    );

    std::thread::sleep(Duration::from_secs(2));
}
```

## cancel callback

```rust
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
    let count = Arc::new(AtomicUsize::new(1));
    let count_clone = count.clone();
    let cancel_timeout = timer.set_timeout(
        move || {
            count_clone.fetch_add(1, SeqCst);
            println!("run callback success");
        },
        Duration::from_secs(1),
    );

    sleep(Duration::from_millis(20));
    // cancel timeout callback;
    cancel_timeout();
    sleep(Duration::from_secs(1));
    // count still be 1;
    assert_eq!(count.load(SeqCst), 1);
}
```

## use async function;

```rust
use std::time::Duration;

use swnb_timer::Timer;

async fn main() {
    let timer = Timer::new();

    // print count every 1 sec
    let async_block = async {
        let mut count = 1;
        loop {
            timer.wait(Duration::from_secs(1)).await;
            count += 1;
            println!("{count}");
        }
    };

    async_block.await;
}

```
