use chrono::{DateTime, Utc};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    #[test]
    fn system_clock_advances() {
        let c = SystemClock;
        let a = c.now();
        thread::sleep(Duration::from_millis(2));
        let b = c.now();
        assert!(b > a);
    }
}

