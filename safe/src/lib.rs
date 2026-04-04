#![deny(unsafe_op_in_unsafe_fn)]

pub mod abi;
pub mod pixels;
pub mod runtime;
pub mod simd;
pub(crate) mod ops;

pub use abi::basic::*;
pub use abi::connection::*;
pub use abi::image::*;
pub use abi::object::*;
pub use abi::operation::*;
pub use abi::r#type::*;
pub use abi::region::*;
pub use runtime::buf::*;
pub use runtime::cache::*;
pub use runtime::connection::*;
pub use runtime::dbuf::*;
pub use runtime::error::*;
pub use runtime::generate::*;
pub use runtime::header::*;
pub use runtime::image::*;
pub use runtime::init::*;
pub use runtime::memory::*;
pub use runtime::object::*;
pub use runtime::operation::*;
pub use runtime::r#type::*;
pub use runtime::rect::*;
pub use runtime::region::*;
pub use runtime::sbuf::*;
pub use runtime::source::*;
pub use runtime::target::*;
pub use runtime::threadpool::*;
pub use runtime::vips_native::*;

pub const VIPS_SAFE_SCAFFOLD_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn scaffold_version() -> &'static str {
    VIPS_SAFE_SCAFFOLD_VERSION
}
