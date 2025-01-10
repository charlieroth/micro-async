use core::{
    cell::{RefCell, RefMut},
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU32, Ordering},
    task::{Context, Poll},
};
use critical_section::Mutex;
use fugit::{Duration, Instant};
use heapless::{binary_heap::Min, BinaryHeap};
use microbit::{
    hal::{
        rtc::{RtcCompareReg, RtcInterrupt},
        Rtc,
    },
    pac::{interrupt, NVIC, RTC0},
};

use crate::executor::{wake_task, ExtWaker};

type TickInstant = Instant<u64, 1, 32768>;
type TickDuration = Duration<u64, 1, 32768>;

const MAX_DEADLINES: usize = 8;
static WAKE_DEADLINES: Mutex<RefCell<BinaryHeap<(u64, usize), Min, MAX_DEADLINES>>> =
    Mutex::new(RefCell::new(BinaryHeap::new()));

fn schedule_wakeups(
    mut rm_deadlines: RefMut<BinaryHeap<(u64, usize), Min, MAX_DEADLINES>>,
    mut rm_rtc: RefMut<Option<Rtc<RTC0>>>,
) {
    let rtc = rm_rtc.as_mut().unwrap();
    while let Some((deadline, task_id)) = rm_deadlines.peek() {
        let ovf_count = (*deadline >> 24) as u32;
        if ovf_count == TICKER.ovf_count.load(Ordering::Relaxed) {
            let counter = (*deadline & 0xFF_FF_FF) as u32;
            if counter > (rtc.get_counter() + 1) {
                rtc.set_compare(RtcCompareReg::Compare0, counter).ok();
                rtc.enable_event(RtcInterrupt::Compare0);
            } else {
                wake_task(*task_id);
                rm_deadlines.pop();
                continue;
            }
        }
        break;
    }
    if rm_deadlines.is_empty() {
        rtc.disable_event(RtcInterrupt::Compare0);
    }
}

enum TimerState {
    Init,
    Wait,
}

pub struct Timer {
    end_time: TickInstant,
    state: TimerState,
}

impl Timer {
    pub fn new(duration: TickDuration) -> Self {
        Self {
            end_time: Ticker::now() + duration,
            state: TimerState::Init,
        }
    }

    fn register(&self, task_id: usize) {
        let new_deadline = self.end_time.ticks();
        critical_section::with(|cs| {
            let mut rm_deadlines = WAKE_DEADLINES.borrow_ref_mut(cs);
            let is_earliest = if let Some((next_deadline, _)) = rm_deadlines.peek() {
                new_deadline < *next_deadline
            } else {
                true
            };
            if rm_deadlines.push((new_deadline, task_id)).is_err() {
                panic!("Deadline dropped for task {}", task_id);
            }
            if is_earliest {
                schedule_wakeups(rm_deadlines, TICKER.rtc.borrow_ref_mut(cs));
            }
        });
    }
}

impl Future for Timer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            TimerState::Init => {
                self.register(cx.waker().task_id());
                self.state = TimerState::Wait;
                Poll::Pending
            }
            TimerState::Wait => {
                if Ticker::now() >= self.end_time {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

pub async fn delay(duration: TickDuration) {
    Timer::new(duration).await;
}

static TICKER: Ticker = Ticker {
    ovf_count: AtomicU32::new(0),
    rtc: Mutex::new(RefCell::new(None)),
};

/// Keeps track of time for the system using RTC0, ticks at a rate of 32,768/sec
///
/// RTC0's counter is 24-bits wide, meaning overflow every ~8 minutes
pub struct Ticker {
    ovf_count: AtomicU32,
    rtc: Mutex<RefCell<Option<Rtc<RTC0>>>>,
}

impl Ticker {
    /// Create on startup to get RTC0 going. Hoists the HAL representation
    /// of RTC0 into the `static TICKER` where it can be accessed by the
    /// interrupt handler function or any `TickTimer` instance.
    pub fn init(rtc0: RTC0, nvic: &mut NVIC) {
        let mut rtc = Rtc::new(rtc0, 0).unwrap();
        rtc.enable_counter();
        #[cfg(feature = "trigger-overflow")]
        {
            rtc.trigger_overflow();
            // wait for the counter to initialize with its close-to-overflow
            // value before advancing, otherwise one of the tasks could
            // schedule a wakeup that will get skipped over when init happens.
            while rtc.get_counter() == 0 {}
        }
        rtc.enable_event(RtcInterrupt::Overflow);
        rtc.enable_interrupt(RtcInterrupt::Overflow, Some(nvic));
        rtc.enable_interrupt(RtcInterrupt::Compare0, Some(nvic));
        critical_section::with(|cs| {
            TICKER.rtc.replace(cs, Some(rtc));
        });
    }

    /// Current time is expressed as TickConstant, which is a combination of:
    /// - The current RTC0 counter value (lowest 24 bits)
    /// - RTC0 counter overflow count (`ovf_count`, upper 40 bits)
    ///
    /// Extra care is need to ensure that the current overflow-count & counter
    /// value are collected during the same overflow-cycle.
    /// `Ordering::SeqCst` is used to prevent the compiler or processor from
    /// moving things around.
    pub fn now() -> TickInstant {
        let ticks = {
            loop {
                let ovf_before = TICKER.ovf_count.load(Ordering::SeqCst);
                let counter = critical_section::with(|cs| {
                    TICKER.rtc.borrow_ref(cs).as_ref().unwrap().get_counter()
                });
                let ovf = TICKER.ovf_count.load(Ordering::SeqCst);
                if ovf_before == ovf {
                    break ((ovf as u64) << 24 | counter as u64);
                }
            }
        };
        TickInstant::from_ticks(ticks)
    }
}

#[interrupt]
fn RTC0() {
    critical_section::with(|cs| {
        let mut rm_rtc = TICKER.rtc.borrow_ref_mut(cs);
        let rtc = rm_rtc.as_mut().unwrap();
        if rtc.is_event_triggered(RtcInterrupt::Overflow) {
            rtc.reset_event(RtcInterrupt::Overflow);
            TICKER.ovf_count.fetch_add(1, Ordering::Relaxed);
        }
        if rtc.is_event_triggered(RtcInterrupt::Compare0) {
            rtc.reset_event(RtcInterrupt::Compare0);
        }

        // For OVF & COMPARE0 events, schedule the next wakeup. This should also
        // kill enough clock cycles to allow the event flags to clear
        schedule_wakeups(WAKE_DEADLINES.borrow_ref_mut(cs), rm_rtc);
    });
}
