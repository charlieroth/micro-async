#![no_std]
#![no_main]

mod button;
mod channel;
mod led;
mod time;

use button::{ButtonDirection, ButtonTask};
use channel::Channel;
use cortex_m_rt::entry;
use embedded_hal::digital::OutputPin;
use led::LedTask;
use microbit::Board;
use panic_rtt_target as _;
use rtt_target::rtt_init_print;
use time::Ticker;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let ticker = Ticker::new(board.RTC0);

    let left_button = board.buttons.button_a.degrade();
    let right_button = board.buttons.button_b.degrade();

    let (col, mut row) = board.display_pins.degrade();
    row[0].set_high().ok();

    let channel: Channel<ButtonDirection> = Channel::new();
    let mut led_task = LedTask::new(col, &ticker, channel.get_receiver());
    let mut left_button_task = ButtonTask::new(
        left_button,
        &ticker,
        ButtonDirection::Left,
        channel.get_sender(),
    );
    let mut right_button_task = ButtonTask::new(
        right_button,
        &ticker,
        ButtonDirection::Right,
        channel.get_sender(),
    );

    // Create an event loop which polls each task every loop and performs
    // the appropriate state change based on the states of the tasks.
    loop {
        led_task.poll();
        left_button_task.poll();
        right_button_task.poll();
    }
}
