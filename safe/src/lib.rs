#![deny(unsafe_op_in_unsafe_fn)]

pub mod abi;
pub mod runtime;

pub use abi::basic::*;
pub use abi::connection::*;
pub use abi::image::*;
pub use abi::object::*;
pub use abi::operation::*;
pub use abi::region::*;
pub use abi::r#type::*;
pub use runtime::error::*;
pub use runtime::init::*;
pub use runtime::object::*;
pub use runtime::r#type::*;

pub const VIPS_SAFE_SCAFFOLD_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn scaffold_version() -> &'static str {
    VIPS_SAFE_SCAFFOLD_VERSION
}
