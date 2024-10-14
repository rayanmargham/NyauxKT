use hpet::HPET;

use crate::println;
pub mod hpet;
pub mod lapic;
pub fn init_timers<'a>() -> Result<(), &'a str> {
    HPET::new();
    Ok(())
}
