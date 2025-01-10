use core::sync::atomic::{AtomicUsize, Ordering};

use embedded_hal::digital::{InputPin, PinState};
use microbit::{
    hal::{
        gpio::{Floating, Input, Pin},
        gpiote::Gpiote,
    },
    pac::{interrupt, Interrupt, NVIC},
};

use crate::{
    executor::wake_task,
    future::{MicroFuture, MicroPoll},
};

const INVALID_TASK_ID: usize = 0xFFFF_FFFF;
const DEFAULT_TASK: AtomicUsize = AtomicUsize::new(INVALID_TASK_ID);
const MAX_CHANNELS_USED: usize = 2;
static NEXT_CHANNEL: AtomicUsize = AtomicUsize::new(0);
static WAKE_TASKS: [AtomicUsize; MAX_CHANNELS_USED] = [DEFAULT_TASK; MAX_CHANNELS_USED];

pub struct InputChannel {
    pin: Pin<Input<Floating>>,
    channel_id: usize,
    ready_state: PinState,
}

impl InputChannel {
    pub fn new(pin: Pin<Input<Floating>>, gpiote: &Gpiote) -> Self {
        let channel_id = NEXT_CHANNEL.fetch_add(1, Ordering::Relaxed);
        let channel = match channel_id {
            0 => gpiote.channel0(),
            1 => gpiote.channel1(),
            MAX_CHANNELS_USED.. => todo!("Setup up more channels!"),
        };
        channel.input_pin(&pin).toggle().enable_interrupt();
        // NOTE(charlieroth): Not using mask-based critical sections
        unsafe {
            NVIC::unmask(Interrupt::GPIOTE);
        }
        Self {
            pin,
            channel_id,
            ready_state: PinState::Low,
        }
    }

    pub fn set_ready_state(&mut self, ready_state: PinState) {
        self.ready_state = ready_state;
    }
}

impl MicroFuture for InputChannel {
    type Output = ();

    fn poll(&mut self, task_id: usize) -> MicroPoll<Self::Output> {
        if self.ready_state == PinState::from(self.pin.is_high().unwrap()) {
            MicroPoll::Ready(())
        } else {
            WAKE_TASKS[self.channel_id].store(task_id, Ordering::Relaxed);
            MicroPoll::Pending
        }
    }
}

#[interrupt]
fn GPIOTE() {
    // Use limited to `events_in` register which is not accessed elsewhere
    let gpiote = unsafe { &*microbit::pac::GPIOTE::ptr() };
    for (channel, task) in WAKE_TASKS.iter().enumerate() {
        if gpiote.events_in[channel].read().bits() != 0 {
            gpiote.events_in[channel].write(|w| w);
            // Swap in the INVALID_TASK_ID to prevent the task-ready queue from
            // getting filled up during debounce.
            let task_id = task.swap(INVALID_TASK_ID, Ordering::Relaxed);
            if task_id != INVALID_TASK_ID {
                wake_task(task_id);
            }
        }
    }
    // Dummy read to ensure event flags clear
    let _ = gpiote.events_in[0].read().bits();
}
