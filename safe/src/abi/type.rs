use libc::c_char;

use super::basic::VipsCallbackFn;

#[repr(C)]
pub struct VipsThing {
    pub i: libc::c_int,
}

#[repr(C)]
pub struct VipsArea {
    pub data: *mut libc::c_void,
    pub length: usize,
    pub n: libc::c_int,
    pub count: libc::c_int,
    pub lock: *mut glib_sys::GMutex,
    pub free_fn: VipsCallbackFn,
    pub client: *mut libc::c_void,
    pub r#type: glib_sys::GType,
    pub sizeof_type: usize,
}

#[repr(C)]
pub struct VipsSaveString {
    pub s: *mut c_char,
}

#[repr(C)]
pub struct VipsRefString {
    pub area: VipsArea,
}

#[repr(C)]
pub struct VipsBlob {
    pub area: VipsArea,
}

#[repr(C)]
pub struct VipsArrayDouble {
    pub area: VipsArea,
}

#[repr(C)]
pub struct VipsArrayInt {
    pub area: VipsArea,
}

#[repr(C)]
pub struct VipsArrayImage {
    pub area: VipsArea,
}
