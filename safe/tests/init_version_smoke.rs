use std::ffi::{CStr, CString};

#[test]
fn init_version_and_error_exports_smoke() {
    let argv0 = CString::new("/tmp/init_version_smoke").expect("argv0");

    assert_eq!(vips::vips_init(argv0.as_ptr()), 0);
    assert_eq!(vips::vips_init(argv0.as_ptr()), 0);

    assert_eq!(vips::vips_version(0), 8);
    assert_eq!(vips::vips_version(1), 15);
    assert_eq!(vips::vips_version(2), 1);
    assert_eq!(vips::vips_version(3), 59);
    assert_eq!(vips::vips_version(4), 1);
    assert_eq!(vips::vips_version(5), 17);

    let version = unsafe { CStr::from_ptr(vips::vips_version_string()) }
        .to_str()
        .expect("version string");
    assert_eq!(version, "8.15.1");

    let argv0_copy = unsafe { CStr::from_ptr(vips::vips_get_argv0()) }
        .to_str()
        .expect("argv0");
    assert_eq!(argv0_copy, "/tmp/init_version_smoke");

    let prgname = unsafe { CStr::from_ptr(vips::vips_get_prgname()) }
        .to_str()
        .expect("prgname");
    assert_eq!(prgname, "init_version_smoke");

    vips::vips_error_clear();
    assert_eq!(vips::vips_version(99), -1);
    let error_text = unsafe { CStr::from_ptr(vips::vips_error_buffer()) }
        .to_str()
        .expect("error buffer");
    assert!(error_text.contains("vips_version"));
    assert!(error_text.contains("flag not in [0, 5]"));

    let copied = vips::vips_error_buffer_copy();
    let copied_text = unsafe { CStr::from_ptr(copied) }
        .to_str()
        .expect("copied error buffer")
        .to_owned();
    unsafe {
        glib_sys::g_free(copied.cast());
    }
    assert!(copied_text.contains("vips_version"));
    assert_eq!(
        unsafe { CStr::from_ptr(vips::vips_error_buffer()) }
            .to_str()
            .expect("cleared buffer"),
        "",
    );

    vips::vips_error_freeze();
    assert_eq!(vips::vips_version(99), -1);
    assert_eq!(
        unsafe { CStr::from_ptr(vips::vips_error_buffer()) }
            .to_str()
            .expect("frozen buffer"),
        "",
    );
    vips::vips_error_thaw();

    vips::vips_shutdown();
    vips::vips_shutdown();
}
