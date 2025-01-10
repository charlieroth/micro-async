#![no_std]
#![no_main]

mod button;
mod channel;
mod executor;
mod future;
mod gpiote;
mod led;
mod time;

use button::{ButtonDirection, ButtonTask};
use channel::Channel;
use cortex_m_rt::entry;
use embedded_hal::digital::OutputPin;
use future::MicroFuture;
use led::LedTask;
use microbit::{hal::gpiote::Gpiote, Board};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;
use time::Ticker;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let mut board = Board::take().unwrap();
    Ticker::init(board.RTC0, &mut board.NVIC);
    let gpiote = Gpiote::new(board.GPIOTE);

    let left_button = board.buttons.button_a.degrade();
    let right_button = board.buttons.button_b.degrade();

    let (col, mut row) = board.display_pins.degrade();
    row[0].set_high().ok();

    let channel: Channel<ButtonDirection> = Channel::new();
    let mut led_task = LedTask::new(col, channel.get_receiver());
    let mut left_button_task = ButtonTask::new(
        left_button,
        ButtonDirection::Left,
        channel.get_sender(),
        &gpiote,
    );
    let mut right_button_task = ButtonTask::new(
        right_button,
        ButtonDirection::Right,
        channel.get_sender(),
        &gpiote,
    );

    let mut tasks: [&mut dyn MicroFuture<Output = ()>; 3] =
        [&mut led_task, &mut left_button_task, &mut right_button_task];
    executor::run_tasks(&mut tasks);
}
