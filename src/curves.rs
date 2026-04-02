use std::f32::consts::PI;

use bevy::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct CurveEvaluation {
    pub input: f32,
    pub remapped_input: f32,
    pub output: f32,
    pub invalid_input: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ResponseCurve {
    #[default]
    Linear,
    Power {
        exponent: f32,
    },
    Exponential {
        power: f32,
    },
    Logistic {
        midpoint: f32,
        steepness: f32,
    },
    InverseLogistic {
        midpoint: f32,
        steepness: f32,
    },
    Gaussian {
        mean: f32,
        deviation: f32,
    },
    SmoothStep,
    SmootherStep,
    Sine,
    Cosine,
    Step {
        threshold: f32,
    },
    Inverse(Box<ResponseCurve>),
    Remap {
        inner: Box<ResponseCurve>,
        input_min: f32,
        input_max: f32,
    },
    Clamp {
        inner: Box<ResponseCurve>,
        min: f32,
        max: f32,
    },
    Bias {
        inner: Box<ResponseCurve>,
        bias: f32,
    },
}

impl ResponseCurve {
    pub fn sample(&self, input: f32) -> f32 {
        self.evaluate(input).output
    }

    pub fn evaluate(&self, input: f32) -> CurveEvaluation {
        let invalid_input = !input.is_finite();
        let sanitized_input = if invalid_input { 0.0 } else { input };
        let normalized_input = sanitized_input.clamp(0.0, 1.0);

        let (remapped_input, output) = self.evaluate_inner(normalized_input, invalid_input);
        CurveEvaluation {
            input: normalized_input,
            remapped_input,
            output: sanitize_output(output),
            invalid_input,
        }
    }

    pub fn inverse(self) -> Self {
        Self::Inverse(Box::new(self))
    }

    pub fn remap(self, input_min: f32, input_max: f32) -> Self {
        Self::Remap {
            inner: Box::new(self),
            input_min,
            input_max,
        }
    }

    pub fn clamp_input(self, min: f32, max: f32) -> Self {
        Self::Clamp {
            inner: Box::new(self),
            min,
            max,
        }
    }

    pub fn bias(self, bias: f32) -> Self {
        Self::Bias {
            inner: Box::new(self),
            bias,
        }
    }

    fn evaluate_inner(&self, input: f32, invalid_input: bool) -> (f32, f32) {
        if invalid_input {
            return (0.0, 0.0);
        }

        match self {
            ResponseCurve::Linear => (input, input),
            ResponseCurve::Power { exponent } => {
                if !exponent.is_finite() {
                    (input, 0.0)
                } else {
                    (input, input.powf(exponent.max(0.0)))
                }
            }
            ResponseCurve::Exponential { power } => {
                if !power.is_finite() {
                    return (input, 0.0);
                }
                let denominator = power.exp() - 1.0;
                if denominator.abs() <= f32::EPSILON {
                    (input, input)
                } else {
                    (input, ((power * input).exp() - 1.0) / denominator)
                }
            }
            ResponseCurve::Logistic {
                midpoint,
                steepness,
            } => (input, normalized_logistic(input, *midpoint, *steepness)),
            ResponseCurve::InverseLogistic {
                midpoint,
                steepness,
            } => (
                input,
                1.0 - normalized_logistic(input, *midpoint, *steepness),
            ),
            ResponseCurve::Gaussian { mean, deviation } => {
                if !mean.is_finite() || !deviation.is_finite() || *deviation <= f32::EPSILON {
                    (input, 0.0)
                } else {
                    let z = (input - *mean) / *deviation;
                    (input, (-0.5 * z * z).exp())
                }
            }
            ResponseCurve::SmoothStep => (input, input * input * (3.0 - 2.0 * input)),
            ResponseCurve::SmootherStep => (
                input,
                input * input * input * (input * (input * 6.0 - 15.0) + 10.0),
            ),
            ResponseCurve::Sine => (input, (input * PI * 0.5).sin()),
            ResponseCurve::Cosine => (input, 1.0 - (input * PI * 0.5).cos()),
            ResponseCurve::Step { threshold } => {
                (input, if input >= *threshold { 1.0 } else { 0.0 })
            }
            ResponseCurve::Inverse(inner) => {
                let inner_eval = inner.evaluate(input);
                (inner_eval.remapped_input, 1.0 - inner_eval.output)
            }
            ResponseCurve::Remap {
                inner,
                input_min,
                input_max,
            } => {
                let remapped = remap_input(input, *input_min, *input_max);
                let inner_eval = inner.evaluate(remapped);
                (remapped, inner_eval.output)
            }
            ResponseCurve::Clamp { inner, min, max } => {
                let (lo, hi) = ordered_pair(*min, *max);
                let clamped = input.clamp(lo, hi);
                let inner_eval = inner.evaluate(clamped);
                (clamped, inner_eval.output)
            }
            ResponseCurve::Bias { inner, bias } => {
                let inner_eval = inner.evaluate(input);
                let adjusted = apply_bias(inner_eval.output, *bias);
                (inner_eval.remapped_input, adjusted)
            }
        }
    }
}

fn ordered_pair(a: f32, b: f32) -> (f32, f32) {
    if a <= b { (a, b) } else { (b, a) }
}

fn remap_input(input: f32, min: f32, max: f32) -> f32 {
    let (lo, hi) = ordered_pair(min, max);
    if (hi - lo).abs() <= f32::EPSILON {
        0.0
    } else {
        ((input - lo) / (hi - lo)).clamp(0.0, 1.0)
    }
}

fn normalized_logistic(input: f32, midpoint: f32, steepness: f32) -> f32 {
    let slope = if steepness.is_finite() {
        steepness
    } else {
        0.0
    };
    let raw = |x: f32| 1.0 / (1.0 + (-slope * (x - midpoint)).exp());
    let lo = raw(0.0);
    let hi = raw(1.0);
    let value = raw(input);
    if (hi - lo).abs() <= f32::EPSILON {
        value.clamp(0.0, 1.0)
    } else {
        ((value - lo) / (hi - lo)).clamp(0.0, 1.0)
    }
}

fn apply_bias(value: f32, bias: f32) -> f32 {
    let clamped_bias = bias.clamp(0.001, 0.999);
    let divisor = ((1.0 / clamped_bias) - 2.0) * (1.0 - value) + 1.0;
    if divisor.abs() <= f32::EPSILON {
        value
    } else {
        (value / divisor).clamp(0.0, 1.0)
    }
}

fn sanitize_output(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
