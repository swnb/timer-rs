use std::time::Duration;

use swnb_timer::Timer;

fn main() {
    let timer = Timer::new();

    // this future print count every 1 sec
    let async_block = async {
        let mut count = 1;
        loop {
            timer.wait(Duration::from_secs(1)).await;
            count += 1;
            println!("{count}");
        }
    };
}
