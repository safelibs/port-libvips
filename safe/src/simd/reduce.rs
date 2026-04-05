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
    for weight in weights.iter_mut() {
        *weight /= sum;
    }

    let residual = 1.0 - weights.iter().copied().sum::<f64>();
    if residual.abs() > 8.0 * f64::EPSILON {
        if let Some((index, _)) = weights
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
        {
            weights[index] += residual;
        }
    }
}

pub(crate) fn dot(values: &[f64], weights: &[f64]) -> f64 {
    let len = values.len().min(weights.len());
    let lanes = super::lane_width_f64_for_targets(vector::supported_targets()).min(4);
    let mut lane_acc = [0.0; 4];
    let mut index = 0;
    while index + lanes <= len {
        for lane in 0..lanes {
            lane_acc[lane] += values[index + lane] * weights[index + lane];
        }
        index += lanes;
    }
    let mut acc = lane_acc[..lanes].iter().copied().sum::<f64>();
    while index < len {
        acc += values[index] * weights[index];
        index += 1;
    }
    acc
}
