use std::collections::HashMap;
use std::fs;
use std::mem::{offset_of, size_of};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use vips::*;

#[derive(Default)]
struct ProbeData {
    sizes: HashMap<String, usize>,
    offsets: HashMap<(String, String), usize>,
}

impl ProbeData {
    fn size(&self, name: &str) -> usize {
        *self.sizes.get(name).expect("size entry")
    }

    fn offset(&self, type_name: &str, field_name: &str) -> usize {
        *self
            .offsets
            .get(&(type_name.to_owned(), field_name.to_owned()))
            .expect("offset entry")
    }
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn staged_include_dir() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("repo root")
        .join("build-check-install/include")
}

fn probe_data() -> &'static ProbeData {
    static DATA: OnceLock<ProbeData> = OnceLock::new();
    DATA.get_or_init(|| build_and_run_probe())
}

fn build_and_run_probe() -> ProbeData {
    let work_dir = manifest_dir().join("target/abi-layout");
    fs::create_dir_all(&work_dir).expect("create abi layout work dir");

    let source_path = work_dir.join("probe.c");
    let binary_path = work_dir.join("probe");
    fs::write(&source_path, probe_source()).expect("write probe source");

    let pkg_config = Command::new("pkg-config")
        .args(["--cflags", "gio-2.0"])
        .output()
        .expect("run pkg-config");
    assert!(
        pkg_config.status.success(),
        "pkg-config failed: {}",
        String::from_utf8_lossy(&pkg_config.stderr)
    );

    let mut compile = Command::new("cc");
    compile.arg("-std=c99");
    compile.arg(format!("-I{}", staged_include_dir().display()));
    for flag in String::from_utf8(pkg_config.stdout)
        .expect("pkg-config utf8")
        .split_whitespace()
    {
        compile.arg(flag);
    }
    compile.arg(&source_path);
    compile.arg("-o");
    compile.arg(&binary_path);

    let compile_output = compile.output().expect("compile probe");
    assert!(
        compile_output.status.success(),
        "probe compile failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let run_output = Command::new(&binary_path).output().expect("run probe");
    assert!(
        run_output.status.success(),
        "probe run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    parse_probe_output(&String::from_utf8(run_output.stdout).expect("probe utf8"))
}

fn parse_probe_output(output: &str) -> ProbeData {
    let mut data = ProbeData::default();
    for line in output.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["SIZE", type_name, value] => {
                data.sizes
                    .insert((*type_name).to_owned(), value.parse().expect("size"));
            }
            ["OFFSET", type_name, field_name, value] => {
                data.offsets.insert(
                    ((*type_name).to_owned(), (*field_name).to_owned()),
                    value.parse().expect("offset"),
                );
            }
            _ => panic!("unexpected probe line: {line}"),
        }
    }
    data
}

fn probe_source() -> String {
    let lines = [
        "#include <stddef.h>",
        "#include <stdio.h>",
        "#include <vips/basic.h>",
        "#include <vips/object.h>",
        "#include <vips/operation.h>",
        "#include <vips/image.h>",
        "#include <vips/region.h>",
        "#include <vips/connection.h>",
        "#include <vips/type.h>",
        "#include <vips/foreign.h>",
        "#include <vips/format.h>",
        "#include <vips/interpolate.h>",
        "#include <vips/sbuf.h>",
        "#include <vips/threadpool.h>",
        "#include <vips/rect.h>",
        "#include <vips/private.h>",
        "#define PRINT_SIZE(type) printf(\"SIZE %s %zu\\n\", #type, sizeof(type))",
        "#define PRINT_OFFSET(type, field) printf(\"OFFSET %s %s %zu\\n\", #type, #field, offsetof(type, field))",
        "int main(void) {",
        "PRINT_SIZE(VipsRect);",
        "PRINT_OFFSET(VipsRect, left);",
        "PRINT_OFFSET(VipsRect, top);",
        "PRINT_OFFSET(VipsRect, width);",
        "PRINT_OFFSET(VipsRect, height);",
        "PRINT_SIZE(VipsThing);",
        "PRINT_OFFSET(VipsThing, i);",
        "PRINT_SIZE(VipsArea);",
        "PRINT_OFFSET(VipsArea, data);",
        "PRINT_OFFSET(VipsArea, length);",
        "PRINT_OFFSET(VipsArea, n);",
        "PRINT_OFFSET(VipsArea, count);",
        "PRINT_OFFSET(VipsArea, lock);",
        "PRINT_OFFSET(VipsArea, free_fn);",
        "PRINT_OFFSET(VipsArea, client);",
        "PRINT_OFFSET(VipsArea, type);",
        "PRINT_OFFSET(VipsArea, sizeof_type);",
        "PRINT_SIZE(VipsSaveString);",
        "PRINT_OFFSET(VipsSaveString, s);",
        "PRINT_SIZE(VipsRefString);",
        "PRINT_OFFSET(VipsRefString, area);",
        "PRINT_SIZE(VipsBlob);",
        "PRINT_OFFSET(VipsBlob, area);",
        "PRINT_SIZE(VipsArrayDouble);",
        "PRINT_OFFSET(VipsArrayDouble, area);",
        "PRINT_SIZE(VipsArrayInt);",
        "PRINT_OFFSET(VipsArrayInt, area);",
        "PRINT_SIZE(VipsArrayImage);",
        "PRINT_OFFSET(VipsArrayImage, area);",
        "PRINT_SIZE(VipsArgument);",
        "PRINT_OFFSET(VipsArgument, pspec);",
        "PRINT_SIZE(VipsArgumentClass);",
        "PRINT_OFFSET(VipsArgumentClass, parent);",
        "PRINT_OFFSET(VipsArgumentClass, object_class);",
        "PRINT_OFFSET(VipsArgumentClass, flags);",
        "PRINT_OFFSET(VipsArgumentClass, priority);",
        "PRINT_OFFSET(VipsArgumentClass, offset);",
        "PRINT_SIZE(VipsArgumentInstance);",
        "PRINT_OFFSET(VipsArgumentInstance, parent);",
        "PRINT_OFFSET(VipsArgumentInstance, argument_class);",
        "PRINT_OFFSET(VipsArgumentInstance, object);",
        "PRINT_OFFSET(VipsArgumentInstance, assigned);",
        "PRINT_OFFSET(VipsArgumentInstance, close_id);",
        "PRINT_OFFSET(VipsArgumentInstance, invalidate_id);",
        "PRINT_SIZE(VipsObject);",
        "PRINT_OFFSET(VipsObject, parent_instance);",
        "PRINT_OFFSET(VipsObject, constructed);",
        "PRINT_OFFSET(VipsObject, static_object);",
        "PRINT_OFFSET(VipsObject, argument_table);",
        "PRINT_OFFSET(VipsObject, nickname);",
        "PRINT_OFFSET(VipsObject, description);",
        "PRINT_OFFSET(VipsObject, preclose);",
        "PRINT_OFFSET(VipsObject, close);",
        "PRINT_OFFSET(VipsObject, postclose);",
        "PRINT_OFFSET(VipsObject, local_memory);",
        "PRINT_SIZE(VipsObjectClass);",
        "PRINT_OFFSET(VipsObjectClass, parent_class);",
        "PRINT_OFFSET(VipsObjectClass, build);",
        "PRINT_OFFSET(VipsObjectClass, postbuild);",
        "PRINT_OFFSET(VipsObjectClass, summary_class);",
        "PRINT_OFFSET(VipsObjectClass, summary);",
        "PRINT_OFFSET(VipsObjectClass, dump);",
        "PRINT_OFFSET(VipsObjectClass, sanity);",
        "PRINT_OFFSET(VipsObjectClass, rewind);",
        "PRINT_OFFSET(VipsObjectClass, preclose);",
        "PRINT_OFFSET(VipsObjectClass, close);",
        "PRINT_OFFSET(VipsObjectClass, postclose);",
        "PRINT_OFFSET(VipsObjectClass, new_from_string);",
        "PRINT_OFFSET(VipsObjectClass, to_string);",
        "PRINT_OFFSET(VipsObjectClass, output_needs_arg);",
        "PRINT_OFFSET(VipsObjectClass, output_to_arg);",
        "PRINT_OFFSET(VipsObjectClass, nickname);",
        "PRINT_OFFSET(VipsObjectClass, description);",
        "PRINT_OFFSET(VipsObjectClass, argument_table);",
        "PRINT_OFFSET(VipsObjectClass, argument_table_traverse);",
        "PRINT_OFFSET(VipsObjectClass, argument_table_traverse_gtype);",
        "PRINT_OFFSET(VipsObjectClass, deprecated);",
        "PRINT_OFFSET(VipsObjectClass, _vips_reserved1);",
        "PRINT_OFFSET(VipsObjectClass, _vips_reserved2);",
        "PRINT_OFFSET(VipsObjectClass, _vips_reserved3);",
        "PRINT_OFFSET(VipsObjectClass, _vips_reserved4);",
        "PRINT_SIZE(VipsOperation);",
        "PRINT_OFFSET(VipsOperation, parent_instance);",
        "PRINT_OFFSET(VipsOperation, hash);",
        "PRINT_OFFSET(VipsOperation, found_hash);",
        "PRINT_OFFSET(VipsOperation, pixels);",
        "PRINT_SIZE(VipsOperationClass);",
        "PRINT_OFFSET(VipsOperationClass, parent_class);",
        "PRINT_OFFSET(VipsOperationClass, usage);",
        "PRINT_OFFSET(VipsOperationClass, get_flags);",
        "PRINT_OFFSET(VipsOperationClass, flags);",
        "PRINT_OFFSET(VipsOperationClass, invalidate);",
        "PRINT_SIZE(VipsProgress);",
        "PRINT_OFFSET(VipsProgress, im);",
        "PRINT_OFFSET(VipsProgress, run);",
        "PRINT_OFFSET(VipsProgress, eta);",
        "PRINT_OFFSET(VipsProgress, tpels);",
        "PRINT_OFFSET(VipsProgress, npels);",
        "PRINT_OFFSET(VipsProgress, percent);",
        "PRINT_OFFSET(VipsProgress, start);",
        "PRINT_SIZE(VipsImage);",
        "PRINT_OFFSET(VipsImage, parent_instance);",
        "PRINT_OFFSET(VipsImage, Xsize);",
        "PRINT_OFFSET(VipsImage, Ysize);",
        "PRINT_OFFSET(VipsImage, Bands);",
        "PRINT_OFFSET(VipsImage, BandFmt);",
        "PRINT_OFFSET(VipsImage, Coding);",
        "PRINT_OFFSET(VipsImage, Type);",
        "PRINT_OFFSET(VipsImage, Xres);",
        "PRINT_OFFSET(VipsImage, Yres);",
        "PRINT_OFFSET(VipsImage, Xoffset);",
        "PRINT_OFFSET(VipsImage, Yoffset);",
        "PRINT_OFFSET(VipsImage, Length);",
        "PRINT_OFFSET(VipsImage, Compression);",
        "PRINT_OFFSET(VipsImage, Level);",
        "PRINT_OFFSET(VipsImage, Bbits);",
        "PRINT_OFFSET(VipsImage, time);",
        "PRINT_OFFSET(VipsImage, Hist);",
        "PRINT_OFFSET(VipsImage, filename);",
        "PRINT_OFFSET(VipsImage, data);",
        "PRINT_OFFSET(VipsImage, kill);",
        "PRINT_OFFSET(VipsImage, Xres_float);",
        "PRINT_OFFSET(VipsImage, Yres_float);",
        "PRINT_OFFSET(VipsImage, mode);",
        "PRINT_OFFSET(VipsImage, dtype);",
        "PRINT_OFFSET(VipsImage, fd);",
        "PRINT_OFFSET(VipsImage, baseaddr);",
        "PRINT_OFFSET(VipsImage, length);",
        "PRINT_OFFSET(VipsImage, magic);",
        "PRINT_OFFSET(VipsImage, start_fn);",
        "PRINT_OFFSET(VipsImage, generate_fn);",
        "PRINT_OFFSET(VipsImage, stop_fn);",
        "PRINT_OFFSET(VipsImage, client1);",
        "PRINT_OFFSET(VipsImage, client2);",
        "PRINT_OFFSET(VipsImage, sslock);",
        "PRINT_OFFSET(VipsImage, regions);",
        "PRINT_OFFSET(VipsImage, dhint);",
        "PRINT_OFFSET(VipsImage, meta);",
        "PRINT_OFFSET(VipsImage, meta_traverse);",
        "PRINT_OFFSET(VipsImage, sizeof_header);",
        "PRINT_OFFSET(VipsImage, windows);",
        "PRINT_OFFSET(VipsImage, upstream);",
        "PRINT_OFFSET(VipsImage, downstream);",
        "PRINT_OFFSET(VipsImage, serial);",
        "PRINT_OFFSET(VipsImage, history_list);",
        "PRINT_OFFSET(VipsImage, progress_signal);",
        "PRINT_OFFSET(VipsImage, file_length);",
        "PRINT_OFFSET(VipsImage, hint_set);",
        "PRINT_OFFSET(VipsImage, delete_on_close);",
        "PRINT_OFFSET(VipsImage, delete_on_close_filename);",
        "PRINT_SIZE(VipsImageClass);",
        "PRINT_OFFSET(VipsImageClass, parent_class);",
        "PRINT_OFFSET(VipsImageClass, preeval);",
        "PRINT_OFFSET(VipsImageClass, eval);",
        "PRINT_OFFSET(VipsImageClass, posteval);",
        "PRINT_OFFSET(VipsImageClass, written);",
        "PRINT_OFFSET(VipsImageClass, invalidate);",
        "PRINT_OFFSET(VipsImageClass, minimise);",
        "PRINT_SIZE(VipsWindow);",
        "PRINT_OFFSET(VipsWindow, ref_count);",
        "PRINT_OFFSET(VipsWindow, im);",
        "PRINT_OFFSET(VipsWindow, top);",
        "PRINT_OFFSET(VipsWindow, height);",
        "PRINT_OFFSET(VipsWindow, data);",
        "PRINT_OFFSET(VipsWindow, baseaddr);",
        "PRINT_OFFSET(VipsWindow, length);",
        "PRINT_SIZE(VipsBufferThread);",
        "PRINT_OFFSET(VipsBufferThread, hash);",
        "PRINT_OFFSET(VipsBufferThread, thread);",
        "PRINT_SIZE(VipsBufferCache);",
        "PRINT_OFFSET(VipsBufferCache, buffers);",
        "PRINT_OFFSET(VipsBufferCache, thread);",
        "PRINT_OFFSET(VipsBufferCache, im);",
        "PRINT_OFFSET(VipsBufferCache, buffer_thread);",
        "PRINT_OFFSET(VipsBufferCache, reserve);",
        "PRINT_OFFSET(VipsBufferCache, n_reserve);",
        "PRINT_SIZE(VipsBuffer);",
        "PRINT_OFFSET(VipsBuffer, ref_count);",
        "PRINT_OFFSET(VipsBuffer, im);",
        "PRINT_OFFSET(VipsBuffer, area);",
        "PRINT_OFFSET(VipsBuffer, done);",
        "PRINT_OFFSET(VipsBuffer, cache);",
        "PRINT_OFFSET(VipsBuffer, buf);",
        "PRINT_OFFSET(VipsBuffer, bsize);",
        "PRINT_SIZE(VipsRegion);",
        "PRINT_OFFSET(VipsRegion, parent_object);",
        "PRINT_OFFSET(VipsRegion, im);",
        "PRINT_OFFSET(VipsRegion, valid);",
        "PRINT_OFFSET(VipsRegion, type);",
        "PRINT_OFFSET(VipsRegion, data);",
        "PRINT_OFFSET(VipsRegion, bpl);",
        "PRINT_OFFSET(VipsRegion, seq);",
        "PRINT_OFFSET(VipsRegion, thread);",
        "PRINT_OFFSET(VipsRegion, window);",
        "PRINT_OFFSET(VipsRegion, buffer);",
        "PRINT_OFFSET(VipsRegion, invalid);",
        "PRINT_SIZE(VipsRegionClass);",
        "PRINT_OFFSET(VipsRegionClass, parent_class);",
        "PRINT_SIZE(VipsConnection);",
        "PRINT_OFFSET(VipsConnection, parent_object);",
        "PRINT_OFFSET(VipsConnection, descriptor);",
        "PRINT_OFFSET(VipsConnection, tracked_descriptor);",
        "PRINT_OFFSET(VipsConnection, close_descriptor);",
        "PRINT_OFFSET(VipsConnection, filename);",
        "PRINT_SIZE(VipsConnectionClass);",
        "PRINT_OFFSET(VipsConnectionClass, parent_class);",
        "PRINT_SIZE(VipsSource);",
        "PRINT_OFFSET(VipsSource, parent_object);",
        "PRINT_OFFSET(VipsSource, decode);",
        "PRINT_OFFSET(VipsSource, have_tested_seek);",
        "PRINT_OFFSET(VipsSource, is_pipe);",
        "PRINT_OFFSET(VipsSource, read_position);",
        "PRINT_OFFSET(VipsSource, length);",
        "PRINT_OFFSET(VipsSource, data);",
        "PRINT_OFFSET(VipsSource, header_bytes);",
        "PRINT_OFFSET(VipsSource, sniff);",
        "PRINT_OFFSET(VipsSource, blob);",
        "PRINT_OFFSET(VipsSource, mmap_baseaddr);",
        "PRINT_OFFSET(VipsSource, mmap_length);",
        "PRINT_SIZE(VipsSourceClass);",
        "PRINT_OFFSET(VipsSourceClass, parent_class);",
        "PRINT_OFFSET(VipsSourceClass, read);",
        "PRINT_OFFSET(VipsSourceClass, seek);",
        "PRINT_SIZE(VipsSourceCustom);",
        "PRINT_OFFSET(VipsSourceCustom, parent_object);",
        "PRINT_SIZE(VipsSourceCustomClass);",
        "PRINT_OFFSET(VipsSourceCustomClass, parent_class);",
        "PRINT_OFFSET(VipsSourceCustomClass, read);",
        "PRINT_OFFSET(VipsSourceCustomClass, seek);",
        "PRINT_SIZE(VipsGInputStream);",
        "PRINT_OFFSET(VipsGInputStream, parent_instance);",
        "PRINT_OFFSET(VipsGInputStream, source);",
        "PRINT_SIZE(VipsGInputStreamClass);",
        "PRINT_OFFSET(VipsGInputStreamClass, parent_class);",
        "PRINT_SIZE(VipsSourceGInputStream);",
        "PRINT_OFFSET(VipsSourceGInputStream, parent_instance);",
        "PRINT_OFFSET(VipsSourceGInputStream, stream);",
        "PRINT_OFFSET(VipsSourceGInputStream, seekable);",
        "PRINT_OFFSET(VipsSourceGInputStream, info);",
        "PRINT_SIZE(VipsSourceGInputStreamClass);",
        "PRINT_OFFSET(VipsSourceGInputStreamClass, parent_class);",
        "PRINT_SIZE(VipsTarget);",
        "PRINT_OFFSET(VipsTarget, parent_object);",
        "PRINT_OFFSET(VipsTarget, memory);",
        "PRINT_OFFSET(VipsTarget, ended);",
        "PRINT_OFFSET(VipsTarget, memory_buffer);",
        "PRINT_OFFSET(VipsTarget, blob);",
        "PRINT_OFFSET(VipsTarget, output_buffer);",
        "PRINT_OFFSET(VipsTarget, write_point);",
        "PRINT_OFFSET(VipsTarget, position);",
        "PRINT_OFFSET(VipsTarget, delete_on_close);",
        "PRINT_OFFSET(VipsTarget, delete_on_close_filename);",
        "PRINT_SIZE(VipsTargetClass);",
        "PRINT_OFFSET(VipsTargetClass, parent_class);",
        "PRINT_OFFSET(VipsTargetClass, write);",
        "PRINT_OFFSET(VipsTargetClass, finish);",
        "PRINT_OFFSET(VipsTargetClass, read);",
        "PRINT_OFFSET(VipsTargetClass, seek);",
        "PRINT_OFFSET(VipsTargetClass, end);",
        "PRINT_SIZE(VipsTargetCustom);",
        "PRINT_OFFSET(VipsTargetCustom, parent_object);",
        "PRINT_SIZE(VipsTargetCustomClass);",
        "PRINT_OFFSET(VipsTargetCustomClass, parent_class);",
        "PRINT_OFFSET(VipsTargetCustomClass, write);",
        "PRINT_OFFSET(VipsTargetCustomClass, finish);",
        "PRINT_OFFSET(VipsTargetCustomClass, read);",
        "PRINT_OFFSET(VipsTargetCustomClass, seek);",
        "PRINT_OFFSET(VipsTargetCustomClass, end);",
        "PRINT_SIZE(VipsSbuf);",
        "PRINT_OFFSET(VipsSbuf, parent_object);",
        "PRINT_OFFSET(VipsSbuf, source);",
        "PRINT_OFFSET(VipsSbuf, input_buffer);",
        "PRINT_OFFSET(VipsSbuf, chars_in_buffer);",
        "PRINT_OFFSET(VipsSbuf, read_point);",
        "PRINT_OFFSET(VipsSbuf, line);",
        "PRINT_SIZE(VipsSbufClass);",
        "PRINT_OFFSET(VipsSbufClass, parent_class);",
        "PRINT_SIZE(VipsFormat);",
        "PRINT_OFFSET(VipsFormat, parent_object);",
        "PRINT_SIZE(VipsFormatClass);",
        "PRINT_OFFSET(VipsFormatClass, parent_class);",
        "PRINT_OFFSET(VipsFormatClass, is_a);",
        "PRINT_OFFSET(VipsFormatClass, header);",
        "PRINT_OFFSET(VipsFormatClass, load);",
        "PRINT_OFFSET(VipsFormatClass, save);",
        "PRINT_OFFSET(VipsFormatClass, get_flags);",
        "PRINT_OFFSET(VipsFormatClass, priority);",
        "PRINT_OFFSET(VipsFormatClass, suffs);",
        "PRINT_SIZE(VipsInterpolate);",
        "PRINT_OFFSET(VipsInterpolate, parent_object);",
        "PRINT_SIZE(VipsInterpolateClass);",
        "PRINT_OFFSET(VipsInterpolateClass, parent_class);",
        "PRINT_OFFSET(VipsInterpolateClass, interpolate);",
        "PRINT_OFFSET(VipsInterpolateClass, get_window_size);",
        "PRINT_OFFSET(VipsInterpolateClass, window_size);",
        "PRINT_OFFSET(VipsInterpolateClass, get_window_offset);",
        "PRINT_OFFSET(VipsInterpolateClass, window_offset);",
        "PRINT_SIZE(VipsForeign);",
        "PRINT_OFFSET(VipsForeign, parent_object);",
        "PRINT_SIZE(VipsForeignClass);",
        "PRINT_OFFSET(VipsForeignClass, parent_class);",
        "PRINT_OFFSET(VipsForeignClass, priority);",
        "PRINT_OFFSET(VipsForeignClass, suffs);",
        "PRINT_SIZE(VipsForeignLoad);",
        "PRINT_OFFSET(VipsForeignLoad, parent_object);",
        "PRINT_OFFSET(VipsForeignLoad, memory);",
        "PRINT_OFFSET(VipsForeignLoad, access);",
        "PRINT_OFFSET(VipsForeignLoad, flags);",
        "PRINT_OFFSET(VipsForeignLoad, fail_on);",
        "PRINT_OFFSET(VipsForeignLoad, fail);",
        "PRINT_OFFSET(VipsForeignLoad, sequential);",
        "PRINT_OFFSET(VipsForeignLoad, out);",
        "PRINT_OFFSET(VipsForeignLoad, real);",
        "PRINT_OFFSET(VipsForeignLoad, nocache);",
        "PRINT_OFFSET(VipsForeignLoad, disc);",
        "PRINT_OFFSET(VipsForeignLoad, error);",
        "PRINT_OFFSET(VipsForeignLoad, revalidate);",
        "PRINT_SIZE(VipsForeignLoadClass);",
        "PRINT_OFFSET(VipsForeignLoadClass, parent_class);",
        "PRINT_OFFSET(VipsForeignLoadClass, is_a);",
        "PRINT_OFFSET(VipsForeignLoadClass, is_a_buffer);",
        "PRINT_OFFSET(VipsForeignLoadClass, is_a_source);",
        "PRINT_OFFSET(VipsForeignLoadClass, get_flags_filename);",
        "PRINT_OFFSET(VipsForeignLoadClass, get_flags);",
        "PRINT_OFFSET(VipsForeignLoadClass, header);",
        "PRINT_OFFSET(VipsForeignLoadClass, load);",
        "PRINT_SIZE(VipsForeignSave);",
        "PRINT_OFFSET(VipsForeignSave, parent_object);",
        "PRINT_OFFSET(VipsForeignSave, strip);",
        "PRINT_OFFSET(VipsForeignSave, keep);",
        "PRINT_OFFSET(VipsForeignSave, profile);",
        "PRINT_OFFSET(VipsForeignSave, background);",
        "PRINT_OFFSET(VipsForeignSave, page_height);",
        "PRINT_OFFSET(VipsForeignSave, in);",
        "PRINT_OFFSET(VipsForeignSave, ready);",
        "PRINT_SIZE(VipsForeignSaveClass);",
        "PRINT_OFFSET(VipsForeignSaveClass, parent_class);",
        "PRINT_OFFSET(VipsForeignSaveClass, saveable);",
        "PRINT_OFFSET(VipsForeignSaveClass, format_table);",
        "PRINT_OFFSET(VipsForeignSaveClass, coding);",
        "PRINT_SIZE(VipsThreadState);",
        "PRINT_OFFSET(VipsThreadState, parent_object);",
        "PRINT_OFFSET(VipsThreadState, im);",
        "PRINT_OFFSET(VipsThreadState, reg);",
        "PRINT_OFFSET(VipsThreadState, pos);",
        "PRINT_OFFSET(VipsThreadState, x);",
        "PRINT_OFFSET(VipsThreadState, y);",
        "PRINT_OFFSET(VipsThreadState, stop);",
        "PRINT_OFFSET(VipsThreadState, a);",
        "PRINT_OFFSET(VipsThreadState, stall);",
        "PRINT_SIZE(VipsThreadStateClass);",
        "PRINT_OFFSET(VipsThreadStateClass, parent_class);",
        "return 0;",
        "}",
    ];
    lines.join("\n")
}

macro_rules! assert_layout {
    ($probe:expr, $ty:ty, $name:literal, [$($field:tt => $c_field:literal),* $(,)?]) => {{
        assert_eq!(size_of::<$ty>(), $probe.size($name), "size mismatch for {}", $name);
        $(
            assert_eq!(
                offset_of!($ty, $field),
                $probe.offset($name, $c_field),
                "offset mismatch for {}.{}",
                $name,
                $c_field,
            );
        )*
    }};
}

#[test]
fn public_abi_layout_matches_installed_headers() {
    let probe = probe_data();

    assert_layout!(probe, VipsRect, "VipsRect", [left => "left", top => "top", width => "width", height => "height"]);
    assert_layout!(probe, VipsThing, "VipsThing", [i => "i"]);
    assert_layout!(probe, VipsArea, "VipsArea", [data => "data", length => "length", n => "n", count => "count", lock => "lock", free_fn => "free_fn", client => "client", r#type => "type", sizeof_type => "sizeof_type"]);
    assert_layout!(probe, VipsSaveString, "VipsSaveString", [s => "s"]);
    assert_layout!(probe, VipsRefString, "VipsRefString", [area => "area"]);
    assert_layout!(probe, VipsBlob, "VipsBlob", [area => "area"]);
    assert_layout!(probe, VipsArrayDouble, "VipsArrayDouble", [area => "area"]);
    assert_layout!(probe, VipsArrayInt, "VipsArrayInt", [area => "area"]);
    assert_layout!(probe, VipsArrayImage, "VipsArrayImage", [area => "area"]);
    assert_layout!(probe, VipsArgument, "VipsArgument", [pspec => "pspec"]);
    assert_layout!(probe, VipsArgumentClass, "VipsArgumentClass", [parent => "parent", object_class => "object_class", flags => "flags", priority => "priority", offset => "offset"]);
    assert_layout!(probe, VipsArgumentInstance, "VipsArgumentInstance", [parent => "parent", argument_class => "argument_class", object => "object", assigned => "assigned", close_id => "close_id", invalidate_id => "invalidate_id"]);
    assert_layout!(probe, VipsObject, "VipsObject", [parent_instance => "parent_instance", constructed => "constructed", static_object => "static_object", argument_table => "argument_table", nickname => "nickname", description => "description", preclose => "preclose", close => "close", postclose => "postclose", local_memory => "local_memory"]);
    assert_layout!(probe, VipsObjectClass, "VipsObjectClass", [parent_class => "parent_class", build => "build", postbuild => "postbuild", summary_class => "summary_class", summary => "summary", dump => "dump", sanity => "sanity", rewind => "rewind", preclose => "preclose", close => "close", postclose => "postclose", new_from_string => "new_from_string", to_string => "to_string", output_needs_arg => "output_needs_arg", output_to_arg => "output_to_arg", nickname => "nickname", description => "description", argument_table => "argument_table", argument_table_traverse => "argument_table_traverse", argument_table_traverse_gtype => "argument_table_traverse_gtype", deprecated => "deprecated", _vips_reserved1 => "_vips_reserved1", _vips_reserved2 => "_vips_reserved2", _vips_reserved3 => "_vips_reserved3", _vips_reserved4 => "_vips_reserved4"]);
    assert_layout!(probe, VipsOperation, "VipsOperation", [parent_instance => "parent_instance", hash => "hash", found_hash => "found_hash", pixels => "pixels"]);
    assert_layout!(probe, VipsOperationClass, "VipsOperationClass", [parent_class => "parent_class", usage => "usage", get_flags => "get_flags", flags => "flags", invalidate => "invalidate"]);
    assert_layout!(probe, VipsProgress, "VipsProgress", [im => "im", run => "run", eta => "eta", tpels => "tpels", npels => "npels", percent => "percent", start => "start"]);
    assert_layout!(probe, VipsImage, "VipsImage", [parent_instance => "parent_instance", Xsize => "Xsize", Ysize => "Ysize", Bands => "Bands", BandFmt => "BandFmt", Coding => "Coding", Type => "Type", Xres => "Xres", Yres => "Yres", Xoffset => "Xoffset", Yoffset => "Yoffset", Length => "Length", Compression => "Compression", Level => "Level", Bbits => "Bbits", time => "time", Hist => "Hist", filename => "filename", data => "data", kill => "kill", Xres_float => "Xres_float", Yres_float => "Yres_float", mode => "mode", dtype => "dtype", fd => "fd", baseaddr => "baseaddr", length => "length", magic => "magic", start_fn => "start_fn", generate_fn => "generate_fn", stop_fn => "stop_fn", client1 => "client1", client2 => "client2", sslock => "sslock", regions => "regions", dhint => "dhint", meta => "meta", meta_traverse => "meta_traverse", sizeof_header => "sizeof_header", windows => "windows", upstream => "upstream", downstream => "downstream", serial => "serial", history_list => "history_list", progress_signal => "progress_signal", file_length => "file_length", hint_set => "hint_set", delete_on_close => "delete_on_close", delete_on_close_filename => "delete_on_close_filename"]);
    assert_layout!(probe, VipsImageClass, "VipsImageClass", [parent_class => "parent_class", preeval => "preeval", eval => "eval", posteval => "posteval", written => "written", invalidate => "invalidate", minimise => "minimise"]);
    assert_layout!(probe, VipsWindow, "VipsWindow", [ref_count => "ref_count", im => "im", top => "top", height => "height", data => "data", baseaddr => "baseaddr", length => "length"]);
    assert_layout!(probe, VipsBufferThread, "VipsBufferThread", [hash => "hash", thread => "thread"]);
    assert_layout!(probe, VipsBufferCache, "VipsBufferCache", [buffers => "buffers", thread => "thread", im => "im", buffer_thread => "buffer_thread", reserve => "reserve", n_reserve => "n_reserve"]);
    assert_layout!(probe, VipsBuffer, "VipsBuffer", [ref_count => "ref_count", im => "im", area => "area", done => "done", cache => "cache", buf => "buf", bsize => "bsize"]);
    assert_layout!(probe, VipsRegion, "VipsRegion", [parent_object => "parent_object", im => "im", valid => "valid", r#type => "type", data => "data", bpl => "bpl", seq => "seq", thread => "thread", window => "window", buffer => "buffer", invalid => "invalid"]);
    assert_layout!(probe, VipsRegionClass, "VipsRegionClass", [parent_class => "parent_class"]);
    assert_layout!(probe, VipsConnection, "VipsConnection", [parent_object => "parent_object", descriptor => "descriptor", tracked_descriptor => "tracked_descriptor", close_descriptor => "close_descriptor", filename => "filename"]);
    assert_layout!(probe, VipsConnectionClass, "VipsConnectionClass", [parent_class => "parent_class"]);
    assert_layout!(probe, VipsSource, "VipsSource", [parent_object => "parent_object", decode => "decode", have_tested_seek => "have_tested_seek", is_pipe => "is_pipe", read_position => "read_position", length => "length", data => "data", header_bytes => "header_bytes", sniff => "sniff", blob => "blob", mmap_baseaddr => "mmap_baseaddr", mmap_length => "mmap_length"]);
    assert_layout!(probe, VipsSourceClass, "VipsSourceClass", [parent_class => "parent_class", read => "read", seek => "seek"]);
    assert_layout!(probe, VipsSourceCustom, "VipsSourceCustom", [parent_object => "parent_object"]);
    assert_layout!(probe, VipsSourceCustomClass, "VipsSourceCustomClass", [parent_class => "parent_class", read => "read", seek => "seek"]);
    assert_layout!(probe, VipsGInputStream, "VipsGInputStream", [parent_instance => "parent_instance", source => "source"]);
    assert_layout!(probe, VipsGInputStreamClass, "VipsGInputStreamClass", [parent_class => "parent_class"]);
    assert_layout!(probe, VipsSourceGInputStream, "VipsSourceGInputStream", [parent_instance => "parent_instance", stream => "stream", seekable => "seekable", info => "info"]);
    assert_layout!(probe, VipsSourceGInputStreamClass, "VipsSourceGInputStreamClass", [parent_class => "parent_class"]);
    assert_layout!(probe, VipsTarget, "VipsTarget", [parent_object => "parent_object", memory => "memory", ended => "ended", memory_buffer => "memory_buffer", blob => "blob", output_buffer => "output_buffer", write_point => "write_point", position => "position", delete_on_close => "delete_on_close", delete_on_close_filename => "delete_on_close_filename"]);
    assert_layout!(probe, VipsTargetClass, "VipsTargetClass", [parent_class => "parent_class", write => "write", finish => "finish", read => "read", seek => "seek", end => "end"]);
    assert_layout!(probe, VipsTargetCustom, "VipsTargetCustom", [parent_object => "parent_object"]);
    assert_layout!(probe, VipsTargetCustomClass, "VipsTargetCustomClass", [parent_class => "parent_class", write => "write", finish => "finish", read => "read", seek => "seek", end => "end"]);
    assert_layout!(probe, VipsSbuf, "VipsSbuf", [parent_object => "parent_object", source => "source", input_buffer => "input_buffer", chars_in_buffer => "chars_in_buffer", read_point => "read_point", line => "line"]);
    assert_layout!(probe, VipsSbufClass, "VipsSbufClass", [parent_class => "parent_class"]);
    assert_layout!(probe, VipsFormat, "VipsFormat", [parent_object => "parent_object"]);
    assert_layout!(probe, VipsFormatClass, "VipsFormatClass", [parent_class => "parent_class", is_a => "is_a", header => "header", load => "load", save => "save", get_flags => "get_flags", priority => "priority", suffs => "suffs"]);
    assert_layout!(probe, VipsInterpolate, "VipsInterpolate", [parent_object => "parent_object"]);
    assert_layout!(probe, VipsInterpolateClass, "VipsInterpolateClass", [parent_class => "parent_class", interpolate => "interpolate", get_window_size => "get_window_size", window_size => "window_size", get_window_offset => "get_window_offset", window_offset => "window_offset"]);
    assert_layout!(probe, VipsForeign, "VipsForeign", [parent_object => "parent_object"]);
    assert_layout!(probe, VipsForeignClass, "VipsForeignClass", [parent_class => "parent_class", priority => "priority", suffs => "suffs"]);
    assert_layout!(probe, VipsForeignLoad, "VipsForeignLoad", [parent_object => "parent_object", memory => "memory", access => "access", flags => "flags", fail_on => "fail_on", fail => "fail", sequential => "sequential", out => "out", real => "real", nocache => "nocache", disc => "disc", error => "error", revalidate => "revalidate"]);
    assert_layout!(probe, VipsForeignLoadClass, "VipsForeignLoadClass", [parent_class => "parent_class", is_a => "is_a", is_a_buffer => "is_a_buffer", is_a_source => "is_a_source", get_flags_filename => "get_flags_filename", get_flags => "get_flags", header => "header", load => "load"]);
    assert_layout!(probe, VipsForeignSave, "VipsForeignSave", [parent_object => "parent_object", strip => "strip", keep => "keep", profile => "profile", background => "background", page_height => "page_height", r#in => "in", ready => "ready"]);
    assert_layout!(probe, VipsForeignSaveClass, "VipsForeignSaveClass", [parent_class => "parent_class", saveable => "saveable", format_table => "format_table", coding => "coding"]);
    assert_layout!(probe, VipsThreadState, "VipsThreadState", [parent_object => "parent_object", im => "im", reg => "reg", pos => "pos", x => "x", y => "y", stop => "stop", a => "a", stall => "stall"]);
    assert_layout!(probe, VipsThreadStateClass, "VipsThreadStateClass", [parent_class => "parent_class"]);
}
