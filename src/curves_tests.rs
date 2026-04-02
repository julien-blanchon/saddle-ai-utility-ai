use super::*;

#[test]
fn linear_curve_clamps_endpoints() {
    assert_eq!(ResponseCurve::Linear.sample(-1.0), 0.0);
    assert_eq!(ResponseCurve::Linear.sample(0.0), 0.0);
    assert_eq!(ResponseCurve::Linear.sample(1.0), 1.0);
    assert_eq!(ResponseCurve::Linear.sample(2.0), 1.0);
}

#[test]
fn gaussian_peaks_near_mean() {
    let curve = ResponseCurve::Gaussian {
        mean: 0.5,
        deviation: 0.15,
    };
    assert!(curve.sample(0.5) > curve.sample(0.25));
    assert!(curve.sample(0.5) > curve.sample(0.75));
}

#[test]
fn smoothstep_is_monotonic() {
    let curve = ResponseCurve::SmoothStep;
    let mut previous = 0.0;
    for step in 1..=20 {
        let value = curve.sample(step as f32 / 20.0);
        assert!(value >= previous);
        previous = value;
    }
}

#[test]
fn invalid_input_returns_zero_and_marks_trace() {
    let evaluation = ResponseCurve::Logistic {
        midpoint: 0.5,
        steepness: 8.0,
    }
    .evaluate(f32::NAN);
    assert_eq!(evaluation.output, 0.0);
    assert!(evaluation.invalid_input);
}

#[test]
fn logistic_curve_hits_expected_endpoints_and_midpoint() {
    let curve = ResponseCurve::Logistic {
        midpoint: 0.5,
        steepness: 10.0,
    };

    assert!(curve.sample(0.0) <= 0.01);
    assert!((curve.sample(0.5) - 0.5).abs() < 0.05);
    assert!(curve.sample(1.0) >= 0.99);
}

#[test]
fn gaussian_curve_is_symmetric_around_mean() {
    let curve = ResponseCurve::Gaussian {
        mean: 0.4,
        deviation: 0.12,
    };

    let left = curve.sample(0.28);
    let right = curve.sample(0.52);
    assert!((left - right).abs() < 0.001);
}

#[test]
fn inverse_curve_mirrors_wrapped_output() {
    let curve = ResponseCurve::SmoothStep.inverse();
    let sample = curve.sample(0.3);
    let wrapped = ResponseCurve::SmoothStep.sample(0.3);
    assert!((sample - (1.0 - wrapped)).abs() < 0.001);
}

#[test]
fn remap_and_clamp_wrappers_adjust_input_domain() {
    let remapped = ResponseCurve::Linear.remap(0.25, 0.75);
    assert_eq!(remapped.sample(0.25), 0.0);
    assert_eq!(remapped.sample(0.75), 1.0);

    let clamped = ResponseCurve::Linear.clamp_input(0.2, 0.6);
    assert_eq!(clamped.sample(0.0), 0.2);
    assert_eq!(clamped.sample(1.0), 0.6);
}

#[test]
fn bias_wrapper_changes_midrange_response_but_stays_in_range() {
    let base = ResponseCurve::Linear.sample(0.4);
    let biased = ResponseCurve::Linear.bias(0.2).sample(0.4);
    assert_ne!(biased, base);
    assert!((0.0..=1.0).contains(&biased));
}
