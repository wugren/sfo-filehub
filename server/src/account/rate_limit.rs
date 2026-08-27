//! 登录限流：内存固定窗口按来源 key（X-Real-IP/peer）计数。
//!
//! 仅用于 `/account/login`（由 sfo-account 的登录 handler 在读取请求体前
//! 调用）。固定窗口实现简单、无新增依赖；窗口初期的瞬时突发由 docker nginx
//! 的 `limit_req burst` 平滑。不做按账号名的限流，避免攻击者刷目标账号配额
//! 造成针对特定账号的锁定。

use sfo_account::LoginRateLimiter;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 超过该条目数时丢弃非当前窗口的旧键，防止长期空窗口积累内存。
const MAX_TRACKED_KEYS: usize = 10_000;

/// 登录按窗口计数的内存限流器。
pub struct FixedWindowLoginLimiter {
    window_secs: u64,
    max_requests: u32,
    state: Mutex<HashMap<String, (u64, u32)>>,
}

impl FixedWindowLoginLimiter {
    /// `window_secs > 0` 且 `max_requests > 0`；配置 0 表示关闭限流时
    /// 由调用方不注入本限流器。
    pub fn new(window_secs: u64, max_requests: u32) -> Self {
        assert!(window_secs > 0, "login rate limit window must be > 0");
        assert!(max_requests > 0, "login rate limit max must be > 0");
        Self {
            window_secs,
            max_requests,
            state: Mutex::new(HashMap::new()),
        }
    }

    fn current_bucket(&self, now_secs: u64) -> u64 {
        now_secs / self.window_secs
    }
}

impl LoginRateLimiter for FixedWindowLoginLimiter {
    fn allow(&self, key: &str) -> bool {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let bucket = self.current_bucket(now_secs);
        let mut entries = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if entries.len() >= MAX_TRACKED_KEYS {
            entries.retain(|_, (b, _)| *b == bucket);
        }
        let entry = entries.entry(key.to_owned()).or_insert((bucket, 0));
        if entry.0 != bucket {
            *entry = (bucket, 0);
        }
        if entry.1 >= self.max_requests {
            return false;
        }
        entry.1 += 1;
        true
    }
}
