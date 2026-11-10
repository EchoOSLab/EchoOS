use echo_arch_cortex_m::cycles_to_ns;

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
