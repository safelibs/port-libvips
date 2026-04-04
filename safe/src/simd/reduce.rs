use super::vector;

pub(crate) fn normalize_weights(weights: &mut [f64]) {
    let sum: f64 = weights.iter().copied().sum();
    if sum.abs() < f64::EPSILON {
        if let Some(first) = weights.first_mut() {
            *first = 1.0;
        }
        for weight in weights.iter_mut().skip(1) {
            *weight = 0.0;
        }
        return;
    }
    for weight in weights {
        *weight /= sum;
    }
}

pub(crate) fn dot(values: &[f64], weights: &[f64]) -> f64 {
    let len = values.len().min(weights.len());
    let lanes = vector::lane_width_f64().max(1);
    let mut acc = 0.0;
    let mut index = 0;
    while index + lanes <= len {
        let mut lane_acc = 0.0;
        for lane in 0..lanes {
            lane_acc += values[index + lane] * weights[index + lane];
        }
        acc += lane_acc;
        index += lanes;
    }
    while index < len {
        acc += values[index] * weights[index];
        index += 1;
    }
    acc
}
