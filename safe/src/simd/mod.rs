pub(crate) mod reduce;
pub(crate) mod vector;

pub(crate) use reduce::{dot, normalize_weights};

pub(crate) fn lane_width_f64_for_targets(targets: i64) -> usize {
    vector::lane_width_f64_for_targets(targets)
}
