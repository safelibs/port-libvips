use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

pub(crate) const TARGET_SCALAR: i64 = 1 << 0;
pub(crate) const TARGET_SIMD128: i64 = 1 << 1;
pub(crate) const TARGET_SIMD256: i64 = 1 << 2;

static ENABLED: AtomicBool = AtomicBool::new(true);
static DISABLED_TARGETS: AtomicI64 = AtomicI64::new(0);

pub(crate) fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub(crate) fn builtin_targets() -> i64 {
    let mut targets = TARGET_SCALAR;

    #[cfg(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "sse2"
        ),
        target_arch = "aarch64",
        target_arch = "wasm32"
    ))]
    {
        targets |= TARGET_SIMD128;
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    {
        targets |= TARGET_SIMD256;
    }

    targets
}

pub(crate) fn supported_targets() -> i64 {
    if !is_enabled() {
        return 0;
    }
    builtin_targets() & !DISABLED_TARGETS.load(Ordering::Relaxed)
}

pub(crate) fn disable_targets(disabled_targets: i64) {
    DISABLED_TARGETS.store(disabled_targets, Ordering::Relaxed);
}

pub(crate) fn target_name(target: i64) -> Option<&'static str> {
    match target {
        TARGET_SCALAR => Some("scalar"),
        TARGET_SIMD128 => Some("simd128"),
        TARGET_SIMD256 => Some("simd256"),
        _ => None,
    }
}

pub(crate) fn best_target(targets: i64) -> i64 {
    if targets & TARGET_SIMD256 != 0 {
        TARGET_SIMD256
    } else if targets & TARGET_SIMD128 != 0 {
        TARGET_SIMD128
    } else {
        TARGET_SCALAR
    }
}

pub(crate) fn lane_width_f64_for_targets(targets: i64) -> usize {
    match best_target(targets) {
        TARGET_SIMD256 => 4,
        TARGET_SIMD128 => 2,
        _ => 1,
    }
}
