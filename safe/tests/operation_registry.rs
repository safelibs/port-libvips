use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::Once;

use serde::Deserialize;
use vips::*;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ManifestEntry {
    depth: i32,
    description: String,
    nickname: String,
    parent: Option<String>,
    type_name: String,
}

#[derive(Debug, Deserialize)]
struct GeneratedTypeManifest {
    parent: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GeneratedArgType {
    value_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GeneratedArgManifest {
    construct: bool,
    default: Option<String>,
    description: String,
    direction: String,
    flags: Vec<String>,
    kind: String,
    long_name: String,
    name: String,
    priority: i32,
    required: bool,
    #[serde(rename = "type")]
    type_info: GeneratedArgType,
}

#[derive(Debug, Deserialize)]
struct GeneratedOperationManifest {
    flags: Vec<String>,
    arguments: Vec<GeneratedArgManifest>,
}

#[derive(Debug, Deserialize)]
struct GeneratedManifest {
    operation_metadata: HashMap<String, GeneratedOperationManifest>,
    type_metadata: HashMap<String, GeneratedTypeManifest>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LiveArgManifest {
    construct: bool,
    default: Option<String>,
    description: String,
    direction: String,
    flags: Vec<String>,
    kind: String,
    long_name: String,
    name: String,
    priority: i32,
    required: bool,
    value_type: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LiveOperationEntry {
    depth: i32,
    description: String,
    nickname: String,
    type_name: String,
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn init_vips() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        assert_eq!(unsafe { vips_init(c"operation_registry".as_ptr()) }, 0);
    });
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> T {
    let path = manifest_dir().join(path);
    serde_json::from_slice(&std::fs::read(path).expect("read manifest")).expect("parse manifest")
}

fn cstring(text: &str) -> CString {
    CString::new(text).expect("cstring")
}

fn type_name_string(type_: glib_sys::GType) -> String {
    unsafe { CStr::from_ptr(gobject_sys::g_type_name(type_)) }
        .to_string_lossy()
        .into_owned()
}

#[derive(Default)]
struct TypeCollector {
    root: glib_sys::GType,
    entries: Vec<ManifestEntry>,
}

unsafe extern "C" fn collect_types_cb(
    type_: glib_sys::GType,
    data: *mut libc::c_void,
) -> *mut libc::c_void {
    let collector = unsafe { &mut *data.cast::<TypeCollector>() };
    let class = unsafe { gobject_sys::g_type_class_ref(type_) }.cast::<VipsObjectClass>();
    let parent = unsafe { gobject_sys::g_type_parent(type_) };
    collector.entries.push(ManifestEntry {
        depth: unsafe { vips_type_depth(type_) },
        description: unsafe { CStr::from_ptr((*class).description) }
            .to_string_lossy()
            .into_owned(),
        nickname: unsafe { CStr::from_ptr((*class).nickname) }
            .to_string_lossy()
            .into_owned(),
        parent: if type_ == collector.root {
            None
        } else {
            Some(type_name_string(parent))
        },
        type_name: type_name_string(type_),
    });
    ptr::null_mut()
}

fn collect_live_entries(root: glib_sys::GType) -> Vec<ManifestEntry> {
    let mut collector = TypeCollector {
        root,
        entries: Vec::new(),
    };
    unsafe {
        vips_type_map_all(
            root,
            Some(collect_types_cb),
            (&mut collector as *mut TypeCollector).cast(),
        );
    }
    collector.entries.sort();
    collector.entries
}

fn argument_flag_names(flags: VipsArgumentFlags) -> Vec<String> {
    let mut out = Vec::new();
    for (name, bit) in [
        ("VIPS_ARGUMENT_REQUIRED", VIPS_ARGUMENT_REQUIRED),
        ("VIPS_ARGUMENT_CONSTRUCT", VIPS_ARGUMENT_CONSTRUCT),
        ("VIPS_ARGUMENT_SET_ONCE", VIPS_ARGUMENT_SET_ONCE),
        ("VIPS_ARGUMENT_SET_ALWAYS", VIPS_ARGUMENT_SET_ALWAYS),
        ("VIPS_ARGUMENT_INPUT", VIPS_ARGUMENT_INPUT),
        ("VIPS_ARGUMENT_OUTPUT", VIPS_ARGUMENT_OUTPUT),
        ("VIPS_ARGUMENT_DEPRECATED", VIPS_ARGUMENT_DEPRECATED),
        ("VIPS_ARGUMENT_MODIFY", VIPS_ARGUMENT_MODIFY),
        ("VIPS_ARGUMENT_NON_HASHABLE", VIPS_ARGUMENT_NON_HASHABLE),
    ] {
        if flags & bit != 0 {
            out.push(name.to_owned());
        }
    }
    out.sort();
    out
}

fn operation_flag_names(flags: VipsOperationFlags) -> Vec<String> {
    let mut out = Vec::new();
    for (name, bit) in [
        ("VIPS_OPERATION_SEQUENTIAL", VIPS_OPERATION_SEQUENTIAL),
        (
            "VIPS_OPERATION_SEQUENTIAL_UNBUFFERED",
            VIPS_OPERATION_SEQUENTIAL_UNBUFFERED,
        ),
        ("VIPS_OPERATION_NOCACHE", VIPS_OPERATION_NOCACHE),
        ("VIPS_OPERATION_DEPRECATED", VIPS_OPERATION_DEPRECATED),
        ("VIPS_OPERATION_UNTRUSTED", VIPS_OPERATION_UNTRUSTED),
        ("VIPS_OPERATION_BLOCKED", VIPS_OPERATION_BLOCKED),
        ("VIPS_OPERATION_REVALIDATE", VIPS_OPERATION_REVALIDATE),
    ] {
        if flags & bit != 0 {
            out.push(name.to_owned());
        }
    }
    out.sort();
    out
}

fn normalize_default(kind: &str, default: Option<String>) -> Option<String> {
    match (kind, default) {
        ("INT", Some(value)) => match value.trim() {
            "VIPS_MAX_COORD" => Some("10000000".to_owned()),
            "-VIPS_MAX_COORD" => Some("-10000000".to_owned()),
            "INT_MAX" => Some(i32::MAX.to_string()),
            "INT_MAX - 1" => Some((i32::MAX - 1).to_string()),
            "INT_MIN" => Some(i32::MIN.to_string()),
            value => Some(value.to_owned()),
        },
        ("DOUBLE", Some(value)) => value.parse::<f64>().ok().map(|value| value.to_string()),
        (_, default) => default,
    }
}

fn project_operations(entries: Vec<ManifestEntry>) -> Vec<LiveOperationEntry> {
    let mut projected = entries
        .into_iter()
        .map(|entry| LiveOperationEntry {
            depth: entry.depth,
            description: entry.description,
            nickname: entry.nickname,
            type_name: entry.type_name,
        })
        .collect::<Vec<_>>();
    projected.sort();
    projected
}

fn pspec_kind(pspec: *mut gobject_sys::GParamSpec) -> String {
    let value_type = unsafe { (*pspec).value_type };
    if value_type == gobject_sys::G_TYPE_BOOLEAN {
        "BOOL"
    } else if value_type == gobject_sys::G_TYPE_INT {
        "INT"
    } else if value_type == gobject_sys::G_TYPE_UINT64 {
        "UINT64"
    } else if value_type == gobject_sys::G_TYPE_DOUBLE {
        "DOUBLE"
    } else if value_type == gobject_sys::G_TYPE_STRING {
        "STRING"
    } else if value_type == gobject_sys::G_TYPE_POINTER {
        "POINTER"
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_OBJECT) }
        != glib_sys::GFALSE
    {
        "OBJECT"
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_BOXED) }
        != glib_sys::GFALSE
    {
        "BOXED"
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_ENUM) }
        != glib_sys::GFALSE
    {
        "ENUM"
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_FLAGS) }
        != glib_sys::GFALSE
    {
        "FLAGS"
    } else {
        "POINTER"
    }
    .to_owned()
}

fn enum_value_name(type_: glib_sys::GType, value: i32) -> Option<String> {
    let class = unsafe { gobject_sys::g_type_class_ref(type_) }.cast::<gobject_sys::GEnumClass>();
    if class.is_null() {
        return None;
    }
    for index in 0..unsafe { (*class).n_values } {
        let item = unsafe { *(*class).values.add(index as usize) };
        if item.value == value {
            return Some(
                unsafe { CStr::from_ptr(item.value_name) }
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    None
}

fn flags_value_name(type_: glib_sys::GType, value: u32) -> Option<String> {
    let class = unsafe { gobject_sys::g_type_class_ref(type_) }.cast::<gobject_sys::GFlagsClass>();
    if class.is_null() {
        return None;
    }
    for index in 0..unsafe { (*class).n_values } {
        let item = unsafe { *(*class).values.add(index as usize) };
        if item.value == value {
            return Some(
                unsafe { CStr::from_ptr(item.value_name) }
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    if value == 0 {
        for index in 0..unsafe { (*class).n_values } {
            let item = unsafe { *(*class).values.add(index as usize) };
            if item.value == 0 {
                return Some(
                    unsafe { CStr::from_ptr(item.value_name) }
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        return Some("0".to_owned());
    }
    let mut names = Vec::new();
    for index in 0..unsafe { (*class).n_values } {
        let item = unsafe { *(*class).values.add(index as usize) };
        if item.value != 0 && (value & item.value) == item.value {
            names.push(
                unsafe { CStr::from_ptr(item.value_name) }
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    if names.is_empty() {
        None
    } else {
        Some(names.join(" | "))
    }
}

fn pspec_default(pspec: *mut gobject_sys::GParamSpec) -> Option<String> {
    let value_type = unsafe { (*pspec).value_type };
    if value_type == gobject_sys::G_TYPE_BOOLEAN {
        let value = unsafe { (*(pspec.cast::<gobject_sys::GParamSpecBoolean>())).default_value };
        Some(
            if value == glib_sys::GFALSE {
                "FALSE"
            } else {
                "TRUE"
            }
            .to_owned(),
        )
    } else if value_type == gobject_sys::G_TYPE_INT {
        Some(unsafe { (*(pspec.cast::<gobject_sys::GParamSpecInt>())).default_value }.to_string())
    } else if value_type == gobject_sys::G_TYPE_UINT64 {
        Some(
            unsafe { (*(pspec.cast::<gobject_sys::GParamSpecUInt64>())).default_value }.to_string(),
        )
    } else if value_type == gobject_sys::G_TYPE_DOUBLE {
        Some(
            unsafe { (*(pspec.cast::<gobject_sys::GParamSpecDouble>())).default_value }.to_string(),
        )
    } else if value_type == gobject_sys::G_TYPE_STRING {
        let value = unsafe { (*(pspec.cast::<gobject_sys::GParamSpecString>())).default_value };
        if value.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(value) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_ENUM) }
        != glib_sys::GFALSE
    {
        enum_value_name(value_type, unsafe {
            (*(pspec.cast::<gobject_sys::GParamSpecEnum>())).default_value
        })
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_FLAGS) }
        != glib_sys::GFALSE
    {
        flags_value_name(value_type, unsafe {
            (*(pspec.cast::<gobject_sys::GParamSpecFlags>())).default_value
        })
    } else {
        None
    }
}

#[derive(Default)]
struct ArgCollector {
    args: Vec<LiveArgManifest>,
}

unsafe extern "C" fn collect_args_cb(
    _class: *mut VipsObjectClass,
    pspec: *mut gobject_sys::GParamSpec,
    argument_class: *mut VipsArgumentClass,
    a: *mut libc::c_void,
    _b: *mut libc::c_void,
) -> *mut libc::c_void {
    let collector = unsafe { &mut *a.cast::<ArgCollector>() };
    let flags = unsafe { (*argument_class).flags };
    if flags & (VIPS_ARGUMENT_INPUT | VIPS_ARGUMENT_OUTPUT) == 0 {
        return ptr::null_mut();
    }
    collector.args.push(LiveArgManifest {
        construct: flags & VIPS_ARGUMENT_CONSTRUCT != 0,
        default: normalize_default(&pspec_kind(pspec), pspec_default(pspec)),
        description: unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_blurb(pspec)) }
            .to_string_lossy()
            .into_owned(),
        direction: if flags & VIPS_ARGUMENT_INPUT != 0 {
            "input"
        } else {
            "output"
        }
        .to_owned(),
        flags: argument_flag_names(flags),
        kind: pspec_kind(pspec),
        long_name: unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_nick(pspec)) }
            .to_string_lossy()
            .into_owned(),
        name: unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_name(pspec)) }
            .to_string_lossy()
            .into_owned(),
        priority: unsafe { (*argument_class).priority },
        required: flags & VIPS_ARGUMENT_REQUIRED != 0,
        value_type: Some(type_name_string(unsafe { (*pspec).value_type })),
    });
    ptr::null_mut()
}

fn collect_live_args(type_name: &str) -> Vec<LiveArgManifest> {
    let type_ = with_type_name(type_name, |type_name| unsafe {
        gobject_sys::g_type_from_name(type_name.as_ptr())
    });
    assert_ne!(type_, 0, "missing live type {type_name}");
    let class = unsafe { gobject_sys::g_type_class_ref(type_) }.cast::<VipsObjectClass>();
    let mut collector = ArgCollector::default();
    unsafe {
        vips_argument_class_map(
            class,
            Some(collect_args_cb),
            (&mut collector as *mut ArgCollector).cast(),
            ptr::null_mut(),
        );
    }
    collector.args.sort();
    collector.args
}

fn with_type_name<T>(text: &str, f: impl FnOnce(&CStr) -> T) -> T {
    let text = cstring(text);
    f(&text)
}

fn effective_generated_args(
    manifest: &GeneratedManifest,
    type_name: &str,
) -> Vec<GeneratedArgManifest> {
    let mut args = if let Some(parent) = manifest
        .type_metadata
        .get(type_name)
        .and_then(|meta| meta.parent.as_deref())
    {
        if manifest.operation_metadata.contains_key(parent) {
            effective_generated_args(manifest, parent)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    if let Some(operation) = manifest.operation_metadata.get(type_name) {
        args.extend(operation.arguments.iter().cloned());
    }
    args
}

fn effective_generated_flags(manifest: &GeneratedManifest, type_name: &str) -> Vec<String> {
    let mut flags = if let Some(parent) = manifest
        .type_metadata
        .get(type_name)
        .and_then(|meta| meta.parent.as_deref())
    {
        if manifest.operation_metadata.contains_key(parent) {
            effective_generated_flags(manifest, parent)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    if let Some(operation) = manifest.operation_metadata.get(type_name) {
        for flag in &operation.flags {
            if !flags.contains(flag) {
                flags.push(flag.clone());
            }
        }
    }
    flags.sort();
    flags
}

fn expected_live_args(manifest: &GeneratedManifest, type_name: &str) -> Vec<LiveArgManifest> {
    let mut args = effective_generated_args(manifest, type_name)
        .into_iter()
        .map(|argument| {
            let kind = match argument.kind.as_str() {
                "IMAGE" | "INTERPOLATE" => "OBJECT".to_owned(),
                _ => argument.kind,
            };
            LiveArgManifest {
                construct: argument.construct,
                default: normalize_default(&kind, argument.default),
                description: argument.description,
                direction: argument.direction,
                flags: {
                    let mut flags = argument.flags;
                    flags.sort();
                    flags
                },
                kind,
                long_name: argument.long_name,
                name: argument.name.replace('_', "-"),
                priority: argument.priority,
                required: argument.required,
                value_type: argument.type_info.value_type,
            }
        })
        .collect::<Vec<_>>();
    args.sort();
    args
}

#[test]
fn live_registry_matches_reference_manifests() {
    init_vips();

    let reference_types: HashMap<String, serde_json::Value> = serde_json::from_slice(
        &std::fs::read(manifest_dir().join("reference/types.json")).expect("read types manifest"),
    )
    .expect("parse types manifest");
    let reference_operations: HashMap<String, serde_json::Value> = serde_json::from_slice(
        &std::fs::read(manifest_dir().join("reference/operations.json"))
            .expect("read operations manifest"),
    )
    .expect("parse operations manifest");

    let mut expected_types: Vec<ManifestEntry> =
        serde_json::from_value(reference_types["entries"].clone()).expect("types entries");
    let expected_operations: Vec<ManifestEntry> =
        serde_json::from_value(reference_operations["entries"].clone()).expect("operation entries");
    expected_types.sort();

    let live_types = collect_live_entries(unsafe { vips_object_get_type() });
    let live_operations =
        project_operations(collect_live_entries(unsafe { vips_operation_get_type() }));
    let expected_operations = project_operations(expected_operations);

    assert_eq!(live_types, expected_types, "live type tree mismatch");
    assert_eq!(
        live_operations, expected_operations,
        "live operation metadata mismatch"
    );
}

#[test]
fn live_operation_arguments_match_generated_manifest() {
    init_vips();

    let manifest: GeneratedManifest = read_json("src/generated/operations.json");
    for entry in collect_live_entries(unsafe { vips_operation_get_type() }) {
        let live_args = collect_live_args(&entry.type_name);
        let expected_args = expected_live_args(&manifest, &entry.type_name);
        assert_eq!(
            live_args, expected_args,
            "live arguments mismatch for {}",
            entry.type_name
        );

        let type_ = with_type_name(&entry.type_name, |type_name| unsafe {
            gobject_sys::g_type_from_name(type_name.as_ptr())
        });
        let class = unsafe { gobject_sys::g_type_class_ref(type_) }.cast::<VipsOperationClass>();
        let live_flags = operation_flag_names(unsafe { (*class).flags });
        let expected_flags = effective_generated_flags(&manifest, &entry.type_name);
        assert_eq!(
            live_flags, expected_flags,
            "live operation flags mismatch for {}",
            entry.type_name
        );
    }
}
