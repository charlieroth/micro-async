#![no_std]
#![no_main]

mod time;

use cortex_m_rt::entry;
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin, StatefulOutputPin},
};
use microbit::{hal::Timer, Board};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    // Dedicating additional hardware timers will be needed to
    // track time for each delay the program needs to measure.
    // This will not scale well.
    let mut timer = Timer::new(board.TIMER0);

    let mut left_button = board.buttons.button_a.degrade();
    let mut right_button = board.buttons.button_b.degrade();

    let (mut col, mut row) = board.display_pins.degrade();
    row[0].set_high().ok();

    let active_col: usize = 0;
    loop {
        col[active_col].toggle().ok();
        timer.delay_ms(500); // Blocking here

        // Blocking prevents timely detection and response to these
        if left_button.is_low().unwrap() {
            // ...
        }

        if right_button.is_low().unwrap() {
            // ...
        }
    }
}
