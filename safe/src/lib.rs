#![deny(unsafe_op_in_unsafe_fn)]

pub const VIPS_SAFE_SCAFFOLD_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn scaffold_version() -> &'static str {
    VIPS_SAFE_SCAFFOLD_VERSION
}

