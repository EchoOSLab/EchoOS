use echo_bench::Histogram;

#[test]
fn summary_of_one_to_thousand() {
    let mut histogram = Histogram::<1024>::new();
    for value in 1..=1000 {
        assert!(histogram.record(value));
    }

    let summary = histogram.summary().expect("non-empty");
    assert_eq!(summary.samples, 1000);
    assert_eq!(summary.min, 1);
    assert_eq!(summary.max, 1000);
    assert_eq!(summary.p50, 500);
    assert_eq!(summary.p99, 990);
    assert_eq!(summary.p999, 999);
    assert_eq!(summary.mean, 500);
}

#[test]
fn empty_histogram_has_no_summary() {
    let mut histogram = Histogram::<8>::new();
    assert!(histogram.is_empty());
    assert_eq!(histogram.summary(), None);
}

#[test]
fn full_histogram_rejects_samples() {
    let mut histogram = Histogram::<2>::new();
    assert!(histogram.record(1));
    assert!(histogram.record(2));
    assert!(histogram.is_full());
    assert!(!histogram.record(3));
}

#[test]
fn single_sample_summary() {
    let mut histogram = Histogram::<4>::new();
    histogram.record(42);
    let summary = histogram.summary().expect("non-empty");
    assert_eq!(summary.min, 42);
    assert_eq!(summary.p50, 42);
    assert_eq!(summary.p99, 42);
    assert_eq!(summary.p999, 42);
    assert_eq!(summary.max, 42);
    assert_eq!(summary.mean, 42);
}

#[test]
fn json_contains_expected_fields() {
    let mut histogram = Histogram::<1024>::new();
    for value in 1..=1000 {
        histogram.record(value);
    }

    let mut out = String::new();
    histogram.write_json(&mut out).expect("write json");
    assert!(out.contains("\"samples\":1000"));
    assert!(out.contains("\"p50\":500"));
    assert!(out.contains("\"p999\":999"));
}

#[test]
fn csv_has_header_and_row() {
    let mut histogram = Histogram::<8>::new();
    histogram.record(10);
    histogram.record(20);

    let mut out = String::new();
    histogram.write_csv(&mut out).expect("write csv");
    let mut lines = out.lines();
    assert_eq!(lines.next(), Some("samples,min,p50,p99,p999,max,mean"));
    assert_eq!(lines.next(), Some("2,10,10,20,20,20,15"));
}
