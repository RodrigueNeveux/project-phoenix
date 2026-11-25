use std::time::Duration;

pub const RETRY_STATUSES: [u16; 8] = [502, 504, 507, 598, 599, 522, 523, 524];

pub fn retries_status(status: u16) -> bool {
    RETRY_STATUSES.contains(&status)
}

#[derive(Debug, Clone)]
pub struct Backoff {
    min: f64,
    max: f64,
    jitter: bool,
    current: f64,
    fails: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new(500., None, true)
    }
}

impl Backoff {
    pub fn new(min_ms: f64, max_ms: Option<f64>, jitter: bool) -> Self {
        Self { min: min_ms, max: max_ms.unwrap_or(10. * min_ms), jitter, current: min_ms, fails: 0 }
    }

    pub fn fails(&self) -> u32 {
        self.fails
    }

    pub fn succeed(&mut self) {
        self.fails = 0;
        self.current = self.min;
    }

    pub fn fail_with(&mut self, random: f64) -> Duration {
        self.fails += 1;
        let mut n = 2. * self.current;
        if self.jitter {
            n *= random;
        }
        self.current = (self.current + n).min(self.max);
        Duration::from_secs_f64(self.current / 1000.)
    }

    pub fn fail(&mut self) -> Duration {
        self.fail_with(rand::random::<f64>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_like_the_client() {
        let mut b = Backoff::default();
        assert_eq!(b.fail_with(1.), Duration::from_millis(1500));
        assert_eq!(b.fail_with(1.), Duration::from_millis(4500));
        assert_eq!(b.fail_with(1.), Duration::from_millis(5000));
        assert_eq!(b.fails(), 3);
        b.succeed();
        assert_eq!(b.fail_with(0.), Duration::from_millis(500));
    }

    #[test]
    fn retry_statuses() {
        assert!(retries_status(502));
        assert!(retries_status(524));
        assert!(!retries_status(500));
        assert!(!retries_status(503));
    }
}
