use echo_arch_cortex_m::{cycles_to_ns, extend_monotonic, wrap_period_ns};

#[test]
fn zero_cycles_is_zero_ns() {
    assert_eq!(cycles_to_ns(0, 480_000_000), 0);
}

#[test]
fn one_second_at_480_mhz() {
    assert_eq!(cycles_to_ns(480_000_000, 480_000_000), 1_000_000_000);
}

#[test]
fn one_cycle_floors_on_host_math() {
    // 1e9 / 480e6 = 2.083... → 2 ns
    assert_eq!(cycles_to_ns(1, 480_000_000), 2);
}

#[test]
fn zero_hz_is_guarded() {
    assert_eq!(cycles_to_ns(123, 0), 0);
}

#[test]
fn full_u32_range_stays_in_u64() {
    let ns = cycles_to_ns(u32::MAX, 480_000_000);
    assert!(ns > 8_000_000_000, "wrap period should be ~8.9s");
    assert!(ns < u64::MAX);
}

#[test]
fn no_wrap_keeps_epoch() {
    assert_eq!(extend_monotonic(100, 5, 200, 480_000_000), (200, 5));
}

#[test]
fn wrap_adds_one_period() {
    let period = wrap_period_ns(480_000_000);
    assert_eq!(period, 8_947_848_533);
    assert_eq!(extend_monotonic(10, 0, 0, 480_000_000), (0, period));
}

#[test]
fn two_wraps_accumulate_epoch() {
    let period = wrap_period_ns(480_000_000);
    let (_, epoch1) = extend_monotonic(10, 0, 0, 480_000_000);
    let (_, epoch2) = extend_monotonic(10, epoch1, 0, 480_000_000);
    assert_eq!(epoch2, period * 2);
}
