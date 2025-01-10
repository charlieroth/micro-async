use embedded_hal::digital::PinState;
use fugit::ExtU64;
use microbit::hal::{
    gpio::{Floating, Input, Pin},
    gpiote::Gpiote,
};

use crate::{
    channel::Sender,
    future::{MicroFuture, MicroPoll},
    gpiote::InputChannel,
    time::Timer,
};

#[derive(Clone, Copy)]
pub enum ButtonDirection {
    Left,
    Right,
}

enum ButtonState {
    WaitForPress,
    Debounce(Timer),
    WaitForRelease,
}

pub struct ButtonTask<'a> {
    input: InputChannel,
    direction: ButtonDirection,
    state: ButtonState,
    sender: Sender<'a, ButtonDirection>,
}

impl<'a> ButtonTask<'a> {
    /// Creates a new `ButtonTask` state machine
    ///
    /// The `ButtonTask` utilizes as `Sender` structure to send
    /// `ButtonDirection`s to the channel
    pub fn new(
        pin: Pin<Input<Floating>>,
        direction: ButtonDirection,
        sender: Sender<'a, ButtonDirection>,
        gpiote: &Gpiote,
    ) -> Self {
        Self {
            input: InputChannel::new(pin, gpiote),
            direction,
            state: ButtonState::WaitForPress,
            sender,
        }
    }
}

impl MicroFuture for ButtonTask<'_> {
    type Output = ();

    fn poll(&mut self, task_id: usize) -> MicroPoll<Self::Output> {
        loop {
            match self.state {
                ButtonState::WaitForPress => {
                    self.input.set_ready_state(PinState::Low);
                    if let MicroPoll::Ready(_) = self.input.poll(task_id) {
                        self.sender.send(self.direction);
                        self.state = ButtonState::Debounce(Timer::new(100.millis()));
                        continue;
                    }
                }
                ButtonState::Debounce(ref mut timer) => {
                    if let MicroPoll::Ready(_) = timer.poll(task_id) {
                        self.state = ButtonState::WaitForRelease;
                        continue;
                    }
                }
                ButtonState::WaitForRelease => {
                    self.input.set_ready_state(PinState::High);
                    if let MicroPoll::Ready(_) = self.input.poll(task_id) {
                        self.state = ButtonState::WaitForPress;
                        continue;
                    }
                }
            }
            break;
        }
        MicroPoll::Pending
    }
}
