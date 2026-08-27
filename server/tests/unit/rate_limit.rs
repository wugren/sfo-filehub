//! 登录限流器单元测试：窗口配额、key 隔离与窗口滚动恢复。

use filehub_server::account::rate_limit::FixedWindowLoginLimiter;
use sfo_account::LoginRateLimiter;

#[test]
fn allows_up_to_max_then_denies_same_key() {
    let limiter = FixedWindowLoginLimiter::new(60, 2);
    assert!(limiter.allow("127.0.0.1"));
    assert!(limiter.allow("127.0.0.1"));
    assert!(!limiter.allow("127.0.0.1"));
}

#[test]
fn different_keys_are_independent() {
    let limiter = FixedWindowLoginLimiter::new(60, 1);
    assert!(limiter.allow("a"));
    assert!(!limiter.allow("a"));
    assert!(limiter.allow("b"));
}

#[tokio::test]
async fn window_rollover_restores_quota() {
    let limiter = FixedWindowLoginLimiter::new(1, 1);
    assert!(limiter.allow("127.0.0.1"));
    assert!(!limiter.allow("127.0.0.1"));
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert!(limiter.allow("127.0.0.1"));
}
