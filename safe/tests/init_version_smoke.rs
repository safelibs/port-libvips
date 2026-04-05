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

    let object_type = vips::vips_object_get_type();
    let image_type = vips::vips_image_get_type();
    let format_type = vips::vips_format_get_type();
    let sbuf_type = vips::vips_sbuf_get_type();
    let foreign_type = vips::vips_foreign_get_type();
    let foreign_load_type = vips::vips_foreign_load_get_type();
    let foreign_save_type = vips::vips_foreign_save_get_type();
    let interpolate_type = vips::vips_interpolate_get_type();
    let thread_state_type = vips::vips_thread_state_get_type();

    for gtype in [
        object_type,
        image_type,
        format_type,
        sbuf_type,
        foreign_type,
        foreign_load_type,
        foreign_save_type,
        interpolate_type,
        thread_state_type,
    ] {
        assert_ne!(gtype, 0);
    }

    unsafe {
        assert_ne!(
            gobject_sys::g_type_is_a(image_type, object_type),
            glib_sys::GFALSE
        );
        assert_ne!(
            gobject_sys::g_type_is_a(format_type, object_type),
            glib_sys::GFALSE
        );
        assert_ne!(
            gobject_sys::g_type_is_a(sbuf_type, object_type),
            glib_sys::GFALSE
        );
        assert_ne!(
            gobject_sys::g_type_is_a(foreign_type, vips::vips_operation_get_type()),
            glib_sys::GFALSE
        );
        assert_ne!(
            gobject_sys::g_type_is_a(foreign_load_type, foreign_type),
            glib_sys::GFALSE
        );
        assert_ne!(
            gobject_sys::g_type_is_a(foreign_save_type, foreign_type),
            glib_sys::GFALSE
        );
        assert_ne!(
            gobject_sys::g_type_is_a(interpolate_type, object_type),
            glib_sys::GFALSE
        );
        assert_ne!(
            gobject_sys::g_type_is_a(thread_state_type, object_type),
            glib_sys::GFALSE
        );
    }

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
