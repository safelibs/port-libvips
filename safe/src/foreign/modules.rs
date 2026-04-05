use std::mem::size_of;
use std::os::raw::c_char;
use std::sync::OnceLock;

use crate::abi::object::VipsObjectClass;
use crate::abi::operation::{
    VipsForeignLoad, VipsForeignLoadClass, VipsForeignSave, VipsForeignSaveClass,
};
use crate::runtime::object;

unsafe extern "C" fn stub_build(_object: *mut crate::abi::object::VipsObject) -> libc::c_int {
    crate::runtime::error::append_message_str(
        "foreign-module",
        "dynamic module operation is a stub",
    );
    -1
}

unsafe fn configure_class(
    klass: glib_sys::gpointer,
    nickname: *const c_char,
    description: *const c_char,
) {
    let class = klass.cast::<VipsObjectClass>();
    unsafe {
        object::prepare_existing_class(class);
        (*class).nickname = nickname;
        (*class).description = description;
        (*class).build = Some(stub_build);
    }
}

macro_rules! module_type {
    ($fn_name:ident, $parent:path, $class_ty:ty, $instance_ty:ty, $name:literal, $nickname:literal, $description:literal) => {
        #[no_mangle]
        pub extern "C" fn $fn_name() -> glib_sys::GType {
            static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
            *ONCE.get_or_init(|| {
                unsafe extern "C" fn class_init(
                    klass: glib_sys::gpointer,
                    _data: glib_sys::gpointer,
                ) {
                    unsafe {
                        configure_class(
                            klass,
                            concat!($nickname, "\0").as_ptr().cast(),
                            concat!($description, "\0").as_ptr().cast(),
                        );
                    }
                }
                object::register_type(
                    $parent(),
                    concat!($name, "\0").as_ptr().cast(),
                    size_of::<$class_ty>(),
                    Some(class_init),
                    size_of::<$instance_ty>(),
                    None,
                    0,
                )
            })
        }
    };
}

module_type!(
    vips_foreign_load_heif_get_type,
    object::vips_foreign_load_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadHeif",
    "heifload_base",
    "load heif"
);
module_type!(
    vips_foreign_load_heif_file_get_type,
    vips_foreign_load_heif_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadHeifFile",
    "heifload",
    "load heif from file"
);
module_type!(
    vips_foreign_load_heif_buffer_get_type,
    vips_foreign_load_heif_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadHeifBuffer",
    "heifload_buffer",
    "load heif from buffer"
);
module_type!(
    vips_foreign_load_heif_source_get_type,
    vips_foreign_load_heif_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadHeifSource",
    "heifload_source",
    "load heif from source"
);
module_type!(
    vips_foreign_save_heif_get_type,
    object::vips_foreign_save_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveHeif",
    "heifsave_base",
    "save heif"
);
module_type!(
    vips_foreign_save_heif_file_get_type,
    vips_foreign_save_heif_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveHeifFile",
    "heifsave",
    "save heif to file"
);
module_type!(
    vips_foreign_save_heif_buffer_get_type,
    vips_foreign_save_heif_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveHeifBuffer",
    "heifsave_buffer",
    "save heif to buffer"
);
module_type!(
    vips_foreign_save_heif_target_get_type,
    vips_foreign_save_heif_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveHeifTarget",
    "heifsave_target",
    "save heif to target"
);
module_type!(
    vips_foreign_save_avif_file_get_type,
    vips_foreign_save_heif_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveAvifFile",
    "avifsave",
    "save avif to file"
);
module_type!(
    vips_foreign_save_avif_target_get_type,
    vips_foreign_save_heif_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveAvifTarget",
    "avifsave_target",
    "save avif to target"
);

module_type!(
    vips_foreign_load_jxl_get_type,
    object::vips_foreign_load_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadJxl",
    "jxlload_base",
    "load jxl"
);
module_type!(
    vips_foreign_load_jxl_file_get_type,
    vips_foreign_load_jxl_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadJxlFile",
    "jxlload",
    "load jxl from file"
);
module_type!(
    vips_foreign_load_jxl_buffer_get_type,
    vips_foreign_load_jxl_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadJxlBuffer",
    "jxlload_buffer",
    "load jxl from buffer"
);
module_type!(
    vips_foreign_load_jxl_source_get_type,
    vips_foreign_load_jxl_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadJxlSource",
    "jxlload_source",
    "load jxl from source"
);
module_type!(
    vips_foreign_save_jxl_get_type,
    object::vips_foreign_save_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveJxl",
    "jxlsave_base",
    "save jxl"
);
module_type!(
    vips_foreign_save_jxl_file_get_type,
    vips_foreign_save_jxl_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveJxlFile",
    "jxlsave",
    "save jxl to file"
);
module_type!(
    vips_foreign_save_jxl_buffer_get_type,
    vips_foreign_save_jxl_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveJxlBuffer",
    "jxlsave_buffer",
    "save jxl to buffer"
);
module_type!(
    vips_foreign_save_jxl_target_get_type,
    vips_foreign_save_jxl_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveJxlTarget",
    "jxlsave_target",
    "save jxl to target"
);

module_type!(
    vips_foreign_load_magick_get_type,
    object::vips_foreign_load_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadMagick",
    "magickload_base",
    "load via magick"
);
module_type!(
    vips_foreign_load_magick_file_get_type,
    vips_foreign_load_magick_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadMagickFile",
    "magickload",
    "load via magick file"
);
module_type!(
    vips_foreign_load_magick_buffer_get_type,
    vips_foreign_load_magick_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadMagickBuffer",
    "magickload_buffer",
    "load via magick buffer"
);
module_type!(
    vips_foreign_load_magick7_get_type,
    object::vips_foreign_load_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadMagick7",
    "magickload_base",
    "load via magick7"
);
module_type!(
    vips_foreign_load_magick7_file_get_type,
    vips_foreign_load_magick7_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadMagick7File",
    "magickload",
    "load via magick7 file"
);
module_type!(
    vips_foreign_load_magick7_buffer_get_type,
    vips_foreign_load_magick7_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadMagick7Buffer",
    "magickload_buffer",
    "load via magick7 buffer"
);
module_type!(
    vips_foreign_save_magick_get_type,
    object::vips_foreign_save_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveMagick",
    "magicksave_base",
    "save via magick"
);
module_type!(
    vips_foreign_save_magick_file_get_type,
    vips_foreign_save_magick_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveMagickFile",
    "magicksave",
    "save via magick file"
);
module_type!(
    vips_foreign_save_magick_buffer_get_type,
    vips_foreign_save_magick_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveMagickBuffer",
    "magicksave_buffer",
    "save via magick buffer"
);
module_type!(
    vips_foreign_save_magick_bmp_file_get_type,
    vips_foreign_save_magick_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveMagickBmpFile",
    "magicksave_bmp",
    "save bmp via magick"
);
module_type!(
    vips_foreign_save_magick_bmp_buffer_get_type,
    vips_foreign_save_magick_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveMagickBmpBuffer",
    "magicksave_bmp_buffer",
    "save bmp via magick"
);
module_type!(
    vips_foreign_save_magick_gif_file_get_type,
    vips_foreign_save_magick_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveMagickGifFile",
    "magicksave_gif",
    "save gif via magick"
);
module_type!(
    vips_foreign_save_magick_gif_buffer_get_type,
    vips_foreign_save_magick_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSaveMagickGifBuffer",
    "magicksave_gif_buffer",
    "save gif via magick"
);

module_type!(
    vips_foreign_load_openslide_get_type,
    object::vips_foreign_load_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadOpenslide",
    "openslideload_base",
    "load openslide"
);
module_type!(
    vips_foreign_load_openslide_file_get_type,
    vips_foreign_load_openslide_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadOpenslideFile",
    "openslideload",
    "load openslide from file"
);
module_type!(
    vips_foreign_load_openslide_source_get_type,
    vips_foreign_load_openslide_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadOpenslideSource",
    "openslideload_source",
    "load openslide from source"
);

module_type!(
    vips_foreign_load_pdf_get_type,
    object::vips_foreign_load_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadPdf",
    "pdfload_base",
    "load pdf"
);
module_type!(
    vips_foreign_load_pdf_file_get_type,
    vips_foreign_load_pdf_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadPdfFile",
    "pdfload",
    "load pdf from file"
);
module_type!(
    vips_foreign_load_pdf_buffer_get_type,
    vips_foreign_load_pdf_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadPdfBuffer",
    "pdfload_buffer",
    "load pdf from buffer"
);
module_type!(
    vips_foreign_load_pdf_source_get_type,
    vips_foreign_load_pdf_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoadPdfSource",
    "pdfload_source",
    "load pdf from source"
);

fn register_heif() {
    let _ = vips_foreign_load_heif_get_type();
    let _ = vips_foreign_load_heif_file_get_type();
    let _ = vips_foreign_load_heif_buffer_get_type();
    let _ = vips_foreign_load_heif_source_get_type();
    let _ = vips_foreign_save_heif_get_type();
    let _ = vips_foreign_save_heif_file_get_type();
    let _ = vips_foreign_save_heif_buffer_get_type();
    let _ = vips_foreign_save_heif_target_get_type();
    let _ = vips_foreign_save_avif_file_get_type();
    let _ = vips_foreign_save_avif_target_get_type();
}

fn register_jxl() {
    let _ = vips_foreign_load_jxl_get_type();
    let _ = vips_foreign_load_jxl_file_get_type();
    let _ = vips_foreign_load_jxl_buffer_get_type();
    let _ = vips_foreign_load_jxl_source_get_type();
    let _ = vips_foreign_save_jxl_get_type();
    let _ = vips_foreign_save_jxl_file_get_type();
    let _ = vips_foreign_save_jxl_buffer_get_type();
    let _ = vips_foreign_save_jxl_target_get_type();
}

fn register_magick() {
    let _ = vips_foreign_load_magick_get_type();
    let _ = vips_foreign_load_magick_file_get_type();
    let _ = vips_foreign_load_magick_buffer_get_type();
    let _ = vips_foreign_load_magick7_get_type();
    let _ = vips_foreign_load_magick7_file_get_type();
    let _ = vips_foreign_load_magick7_buffer_get_type();
    let _ = vips_foreign_save_magick_get_type();
    let _ = vips_foreign_save_magick_file_get_type();
    let _ = vips_foreign_save_magick_buffer_get_type();
    let _ = vips_foreign_save_magick_bmp_file_get_type();
    let _ = vips_foreign_save_magick_bmp_buffer_get_type();
    let _ = vips_foreign_save_magick_gif_file_get_type();
    let _ = vips_foreign_save_magick_gif_buffer_get_type();
}

fn register_openslide() {
    let _ = vips_foreign_load_openslide_get_type();
    let _ = vips_foreign_load_openslide_file_get_type();
    let _ = vips_foreign_load_openslide_source_get_type();
}

fn register_pdf() {
    let _ = vips_foreign_load_pdf_get_type();
    let _ = vips_foreign_load_pdf_file_get_type();
    let _ = vips_foreign_load_pdf_buffer_get_type();
    let _ = vips_foreign_load_pdf_source_get_type();
}

pub fn register_all() {
    register_heif();
    register_jxl();
    register_magick();
    register_openslide();
    register_pdf();
}

pub fn try_load_for_operation(name: &str) {
    match name {
        "heifload" | "heifload_base" | "heifload_buffer" | "heifload_source" | "heifsave"
        | "heifsave_base" | "heifsave_buffer" | "heifsave_target" | "avifsave"
        | "avifsave_target" => register_heif(),
        "jxlload" | "jxlload_base" | "jxlload_buffer" | "jxlload_source" | "jxlsave"
        | "jxlsave_base" | "jxlsave_buffer" | "jxlsave_target" => register_jxl(),
        "magickload"
        | "magickload_base"
        | "magickload_buffer"
        | "magicksave"
        | "magicksave_base"
        | "magicksave_buffer"
        | "magicksave_bmp"
        | "magicksave_bmp_buffer"
        | "magicksave_gif"
        | "magicksave_gif_buffer" => register_magick(),
        "openslideload" | "openslideload_base" | "openslideload_source" => register_openslide(),
        "pdfload" | "pdfload_base" | "pdfload_buffer" | "pdfload_source" => register_pdf(),
        _ => {}
    }
}
