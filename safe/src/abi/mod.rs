#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

macro_rules! c_enum {
    (
        $vis:vis type $name:ident {
            $($const_name:ident = $value:expr),+ $(,)?
        }
    ) => {
        $vis type $name = libc::c_int;
        $(
            $vis const $const_name: $name = $value;
        )+
    };
}

pub mod basic;
pub mod connection;
pub mod image;
pub mod object;
pub mod operation;
pub mod region;
#[path = "type.rs"]
pub mod r#type;

pub use basic::*;
pub use connection::*;
pub use image::*;
pub use object::*;
pub use operation::*;
pub use region::*;
pub use r#type::*;
