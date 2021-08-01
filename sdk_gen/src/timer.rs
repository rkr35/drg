use crate::list::List;
use core::fmt::{self, Display, Formatter, Write};

static mut TICKS_PER_SECOND: i64 = 0;

pub unsafe fn initialize_ticks_per_second() {
    common::win::QueryPerformanceFrequency(&mut TICKS_PER_SECOND);
    crate::log!("TICKS_PER_SECOND = {}", FormattedTicks(TICKS_PER_SECOND));
}

fn get_current_tick() -> i64 {
    let mut tick = 0;
    unsafe {
        common::win::QueryPerformanceCounter(&mut tick);
    }
    tick
}

pub struct Timer<A: Display> {
    start_tick: i64,
    action: A,
}

impl<A: Display> Timer<A> {
    pub fn new(action: A) -> Self {
        crate::log!("BEGIN: {}", action);

        Self {
            start_tick: get_current_tick(),
            action,
        }
    }

    pub fn stop(self) {
        let current_tick = get_current_tick();
        let elapsed_ticks = current_tick - self.start_tick;
        crate::log!(
            "END: {} ({} ticks elapsed)",
            self.action,
            FormattedTicks(elapsed_ticks)
        );
    }
}

struct FormattedTicks(i64);

impl Display for FormattedTicks {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        const MAX_TICK_DIGITS: usize = 10;
        let mut digits = List::<u8, MAX_TICK_DIGITS>::new();

        let mut ticks = self.0;

        while ticks > 0 {
            let digit = (ticks % 10) as u8;
            let _ = digits.push(digit);
            ticks /= 10;
        }

        for (i, digit) in digits.iter().enumerate().rev() {
            f.write_char(char::from(b'0' + digit))?;

            let is_final_digit = i == 0;
            let need_comma = !is_final_digit && i % 3 == 0;

            if need_comma {
                f.write_char(',')?;
            }
        }

        Ok(())
    }
}
