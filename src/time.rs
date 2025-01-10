use microbit::{hal::Rtc, pac::RTC0};

type TickInstant = fugit::Instant<u64, 1, 32768>;
type TickDuration = fugit::Duration<u64, 1, 32768>;

pub struct Timer<'a> {
    end_time: TickInstant,
    ticker: &'a Ticker,
}

impl<'a> Timer<'a> {
    pub fn new(duration: TickDuration, ticker: &'a Ticker) -> Self {
        Self {
            end_time: ticker.now() + duration,
            ticker,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ticker.now() >= self.end_time
    }
}

/// Keeps track of time for the system using RTC0, ticks at a rate of 32,768/sec
///
/// RTC0's counter is 24-bits wide, meaning overflow every ~8 minutes
pub struct Ticker {
    rtc: Rtc<RTC0>,
}

impl Ticker {
    /// Create on startup to get RTC0 going
    pub fn new(rtc0: RTC0) -> Self {
        // Use a prescaler of 0 to keep the native frequency of the tick
        let rtc = Rtc::new(rtc0, 0).unwrap();
        rtc.enable_counter();
        Self { rtc }
    }

    pub fn now(&self) -> TickInstant {
        TickInstant::from_ticks(self.rtc.get_counter() as u64)
    }
}
