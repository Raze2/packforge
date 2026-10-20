//! In-memory token-bucket rate limiting per API key (per process).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    Allowed { remaining: u32 },
    Limited { retry_after: Duration },
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    updated: Instant,
}

#[derive(Debug, Default)]
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    /// Takes one token from `key`'s bucket, which holds `per_minute` tokens and refills
    /// continuously at `per_minute` per minute.
    pub fn check(&self, key: &str, per_minute: u32, now: Instant) -> Decision {
        let capacity = f64::from(per_minute.max(1));
        let rate_per_sec = capacity / 60.0;
        let mut buckets = match self.buckets.lock() {
            Ok(b) => b,
            Err(poisoned) => poisoned.into_inner(),
        };
        let b = buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: capacity,
            updated: now,
        });
        let elapsed = now.saturating_duration_since(b.updated).as_secs_f64();
        b.tokens = (b.tokens + elapsed * rate_per_sec).min(capacity);
        b.updated = now;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            Decision::Allowed {
                remaining: b.tokens.floor() as u32,
            }
        } else {
            let wait = (1.0 - b.tokens) / rate_per_sec;
            Decision::Limited {
                retry_after: Duration::from_secs_f64(wait.max(0.001)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_then_refill() {
        let rl = RateLimiter::default();
        let t0 = Instant::now();
        for i in 0..3 {
            assert_eq!(rl.check("k", 3, t0), Decision::Allowed { remaining: 2 - i });
        }
        assert!(matches!(rl.check("k", 3, t0), Decision::Limited { .. }));
        // 3 per minute = one token every 20 s.
        assert!(matches!(
            rl.check("k", 3, t0 + Duration::from_secs(21)),
            Decision::Allowed { .. }
        ));
        // Other keys are independent.
        assert!(matches!(rl.check("other", 3, t0), Decision::Allowed { .. }));
    }
}
