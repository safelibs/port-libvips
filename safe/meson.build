project('vips-safe', 'c', 'cpp',
  version: '8.15.1',
  meson_version: '>=0.57',
  default_options: [
    'c_std=gnu99',
    'cpp_std=c++11',
    'buildtype=release',
    'b_ndebug=if-release',
    'default_library=shared',
  ],
)

pkg = import('pkgconfig')
i18n = import('i18n')

cargo = find_program('cargo')
sh = find_program('sh')
python3 = find_program('python3')
g_ir_scanner = find_program('g-ir-scanner', required: false)
g_ir_compiler = find_program('g-ir-compiler', required: false)
glib_dep = dependency('glib-2.0')
gobject_dep = dependency('gobject-2.0')
gio_dep = dependency('gio-2.0')
gmodule_dep = dependency('gmodule-no-export-2.0', required: false)
if not gmodule_dep.found()
  gmodule_dep = dependency('gmodule-2.0')
endif
cpp = meson.get_compiler('cpp')
cc = meson.get_compiler('c')
dl_dep = cc.find_library('dl', required: false)

if not g_ir_scanner.found() or not g_ir_compiler.found()
  error('g-ir-scanner and g-ir-compiler are required for the compatibility packaging build')
endif

version_parts = meson.project_version().split('.')
version_major = version_parts[0]
version_minor = version_parts[1]
gettext_domain = 'vips@0@.@1@'.format(version_major, version_minor)
library_version = '42.17.1'
soname = 'libvips.so.42'
full_library_name = 'libvips.so.' + library_version
cxx_soname = 'libvips-cpp.so.42'
cxx_full_library_name = 'libvips-cpp.so.' + library_version
module_dir_basename = 'vips-modules-@0@.@1@'.format(version_major, version_minor)

cargo_target_dir = meson.current_source_dir() / 'target'
cargo_artifact = cargo_target_dir / 'release' / 'libvips.so'

feature_options = [
  'modules',
  'introspection',
  'cfitsio',
  'cgif',
  'exif',
  'fftw',
  'fontconfig',
  'archive',
  'heif',
  'heif-module',
  'imagequant',
  'jpeg',
  'jpeg-xl',
  'jpeg-xl-module',
  'lcms',
  'magick',
  'magick-module',
  'matio',
  'nifti',
  'openexr',
  'openjpeg',
  'openslide',
  'openslide-module',
  'highway',
  'orc',
  'pangocairo',
  'pdfium',
  'png',
  'poppler',
  'poppler-module',
  'quantizr',
  'rsvg',
  'spng',
  'tiff',
  'webp',
  'zlib',
]

boolean_options = [
  'deprecated',
  'examples',
  'cplusplus',
  'doxygen',
  'gtk_doc',
  'vapi',
  'nsgif',
  'ppm',
  'analyze',
  'radiance',
]

cargo_feature_args = []
foreach opt : feature_options
  opt_value = get_option(opt)
  if opt_value.enabled()
    cargo_feature_args += ['--features', opt]
  endif
endforeach

foreach opt : boolean_options
  if get_option(opt)
    cargo_feature_args += ['--features', opt]
  endif
endforeach

cargo_env = environment()
cargo_env.set('CARGO_TARGET_DIR', cargo_target_dir)
cargo_env.set('VIPS_SAFE_MAGICK_PACKAGE', get_option('magick-package'))
cargo_env.set('VIPS_SAFE_MAGICK_FEATURES', ','.join(get_option('magick-features')))
cargo_env.set('VIPS_SAFE_NIFTI_PREFIX_DIR', get_option('nifti-prefix-dir'))
cargo_env.set('VIPS_SAFE_MODULE_DIR_BASENAME', module_dir_basename)

cargo_build_cmd = [
  cargo,
  'build',
  '--manifest-path',
  meson.current_source_dir() / 'Cargo.toml',
  '--release',
  '--target-dir',
  cargo_target_dir,
] + cargo_feature_args

libvips_stage = custom_target(
  'libvips-stage',
  output: [
    full_library_name,
    soname,
    'libvips.so',
  ],
  build_by_default: true,
  build_always_stale: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      cargo_out="$1"
      out_real="$2"
      out_soname="$3"
      out_link="$4"
      stage_dir="$5"
      stage_name="$6"
      stage_soname="$7"
      shift 7

      "$@"

      mkdir -p "$stage_dir"
      cp "$cargo_out" "$out_real"
      ln -sf "$(basename "$out_real")" "$out_soname"
      ln -sf "$(basename "$out_soname")" "$out_link"

      cp "$cargo_out" "$stage_dir/$stage_name"
      ln -sf "$stage_name" "$stage_dir/$stage_soname"
      ln -sf "$stage_soname" "$stage_dir/libvips.so"
    ''',
    'meson-cargo-stage',
    cargo_artifact,
    '@OUTPUT0@',
    '@OUTPUT1@',
    '@OUTPUT2@',
    meson.current_build_dir() / 'lib',
    full_library_name,
    soname,
  ] + cargo_build_cmd,
  env: cargo_env,
  install: true,
  install_dir: [
    get_option('libdir'),
    get_option('libdir'),
    get_option('libdir'),
  ],
)

install_subdir(
  'include/vips',
  install_dir: get_option('includedir'),
)

root_dir = meson.current_source_dir() / '..'
original_root = root_dir / 'original'
build_check_install_root = root_dir / 'build-check-install'

pkg.generate(
  filebase: 'vips',
  name: 'vips',
  description: 'Image processing library',
  version: meson.project_version(),
  libraries: ['-lvips'],
  libraries_private: ['-pthread', '-lm'],
  requires: ['glib-2.0 >= 2.52', 'gio-2.0', 'gobject-2.0'],
  requires_private: [
    'gmodule-no-export-2.0',
    'expat',
    'zlib >= 0.4',
    'libjpeg',
    'libpng >= 1.2.9',
    'libwebp >= 0.6',
    'libwebpmux >= 0.6',
    'libwebpdemux >= 0.6',
    'pangocairo >= 1.32.6',
    'pangoft2 >= 1.32.6',
    'fontconfig',
    'libtiff-4',
  ],
  subdirs: '.',
)

install_data(
  build_check_install_root / 'share' / 'locale' / 'de' / 'LC_MESSAGES' / 'vips8.15.mo',
  install_dir: get_option('datadir') / 'locale' / 'de' / 'LC_MESSAGES',
)

install_data(
  build_check_install_root / 'share' / 'locale' / 'en_GB' / 'LC_MESSAGES' / 'vips8.15.mo',
  install_dir: get_option('datadir') / 'locale' / 'en_GB' / 'LC_MESSAGES',
)

install_man(
  original_root / 'man' / 'vipsedit.1',
  original_root / 'man' / 'vipsheader.1',
  original_root / 'man' / 'vips.1',
  original_root / 'man' / 'vipsprofile.1',
  original_root / 'man' / 'vipsthumbnail.1',
)

doc_html_files = files(
  '../original/doc/How-it-works.md',
  '../original/doc/libvips-from-C++.md',
  '../original/doc/Using-vipsthumbnail.md',
  '../original/doc/How-it-opens-files.md',
  '../original/doc/Examples.md',
  '../original/doc/Cite.md',
  '../original/doc/binding.md',
  '../original/doc/Making-image-pyramids.md',
)

gtk_doc_files = files(
  '../original/doc/How-it-works.xml',
  '../original/doc/libvips-from-C++.xml',
  '../original/doc/Using-vipsthumbnail.xml',
  '../original/doc/How-it-opens-files.xml',
  '../original/doc/Examples.xml',
  '../original/doc/Cite.xml',
  '../original/doc/binding.xml',
  '../original/doc/Making-image-pyramids.xml',
  '../original/doc/using-command-line.xml',
  '../original/doc/using-C.xml',
  '../original/doc/using-threads.xml',
  '../original/doc/extending.xml',
  '../original/doc/function-list.xml',
  '../original/doc/file-format.xml',
)

install_data(
  doc_html_files,
  install_dir: get_option('datadir') / 'doc' / 'libvips-doc' / 'html',
)

install_data(
  gtk_doc_files,
  install_dir: get_option('datadir') / 'gtk-doc' / 'html' / 'libvips',
)

install_data(
  [
    original_root / 'doc' / 'images' / 'owl.jpg',
    original_root / 'doc' / 'images' / 'tn_owl.jpg',
    original_root / 'doc' / 'images' / 'interconvert.png',
    original_root / 'doc' / 'images' / 'Combine.png',
    original_root / 'doc' / 'images' / 'Memtrace.png',
    original_root / 'doc' / 'images' / 'Sequence.png',
    original_root / 'doc' / 'images' / 'Sink.png',
    original_root / 'doc' / 'images' / 'Vips-smp.png',
  ],
  install_dir: get_option('datadir') / 'gtk-doc' / 'html' / 'libvips' / 'images',
)

runtime_libdir = meson.current_build_dir() / 'lib'
runtime_link = runtime_libdir / 'libvips.so'
sample_png = root_dir / 'original' / 'test' / 'test-suite' / 'images' / 'sample.png'
vendor_pyvips_dir = meson.current_source_dir() / 'vendor' / 'pyvips-3.1.1'
runtime_module_dir = runtime_libdir / module_dir_basename

test_c_args = ['-DHAVE_UNISTD_H=1']
test_include = include_directories('include')
tool_c_args = [
  '-DGETTEXT_PACKAGE="@0@"'.format(gettext_domain),
  '-DENABLE_DEPRECATED=0',
  '-DENABLE_MODULES=1',
  '-DHAVE_ORC=0',
]
tool_include = include_directories('include', '../original/libvips/include')
cpp_include = include_directories('include', '../original/cplusplus/include')
cpp_export_args = ['-D_VIPS_PUBLIC=__attribute__((visibility("default")))']

libvips_cpp_export_map = custom_target(
  'libvips-cpp-export-map',
  input: meson.current_source_dir() / 'reference' / 'abi' / 'libvips-cpp.symbols',
  output: 'libvips-cpp.export.map',
  build_by_default: true,
  command: [
    python3,
    '-c',
    '''
import pathlib
import sys

source = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
symbols = [
    line.strip()
    for line in source.read_text().splitlines()
    if line.strip() and not line.startswith("#")
]
newline = chr(10)
lines = ["VIPS_CPP_42 {", "  global:"]
lines.extend(f"    {symbol};" for symbol in symbols)
lines.extend(["  local:", "    *;", "};"])
destination.write_text(newline.join(lines) + newline)
''',
    '@INPUT@',
    '@OUTPUT@',
  ],
)

libvips_cpp = shared_library(
  'vips-cpp',
  [
    original_root / 'cplusplus' / 'VImage.cpp',
    original_root / 'cplusplus' / 'VInterpolate.cpp',
    original_root / 'cplusplus' / 'VRegion.cpp',
    original_root / 'cplusplus' / 'VConnection.cpp',
    original_root / 'cplusplus' / 'VError.cpp',
  ],
  build_by_default: true,
  install: true,
  include_directories: cpp_include,
  cpp_args: cpp_export_args,
  dependencies: [glib_dep, gobject_dep, gio_dep],
  version: library_version,
  gnu_symbol_visibility: 'hidden',
  link_args: [
    runtime_link,
    '-Wl,--version-script=' + (meson.current_build_dir() / 'libvips-cpp.export.map'),
  ],
  link_depends: [libvips_stage, libvips_cpp_export_map],
  build_rpath: runtime_libdir,
)

libvips_cpp_stage = custom_target(
  'libvips-cpp-stage',
  input: libvips_cpp,
  output: 'libvips-cpp-stage.stamp',
  build_by_default: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      input="$1"
      stage_dir="$2"
      stage_name="$3"
      stage_soname="$4"
      stamp="$5"

      mkdir -p "$stage_dir"
      cp "$input" "$stage_dir/$stage_name"
      ln -sf "$stage_name" "$stage_dir/$stage_soname"
      ln -sf "$stage_soname" "$stage_dir/libvips-cpp.so"
      : > "$stamp"
    ''',
    'meson-stage-libvips-cpp',
    '@INPUT@',
    runtime_libdir,
    cxx_full_library_name,
    cxx_soname,
    '@OUTPUT@',
  ],
)

pkg.generate(
  filebase: 'vips-cpp',
  name: 'vips-cpp',
  description: 'C++ API for vips8 image processing library',
  version: meson.project_version(),
  libraries: ['-lvips-cpp'],
  libraries_private: ['-pthread', '-lm'],
  requires: ['vips', 'glib-2.0 >= 2.52', 'gobject-2.0'],
  requires_private: [
    'gio-2.0',
    'gmodule-no-export-2.0',
    'expat',
    'zlib >= 0.4',
    'libjpeg',
    'libpng >= 1.2.9',
    'libwebp >= 0.6',
    'libwebpmux >= 0.6',
    'libwebpdemux >= 0.6',
    'pangocairo >= 1.32.6',
    'pangoft2 >= 1.32.6',
    'fontconfig',
    'libtiff-4',
  ],
  subdirs: '.',
)

example_targets = []
foreach example_name : [
  'annotate-animated',
  'new-from-buffer',
  'progress-cancel',
  'use-vips-func',
]
  example_targets += executable(
    example_name,
    original_root / 'examples' / (example_name + '.c'),
    build_by_default: true,
    include_directories: test_include,
    dependencies: [glib_dep, gobject_dep, gio_dep],
    link_args: [runtime_link],
    link_depends: libvips_stage,
    build_rpath: runtime_libdir,
  )
endforeach

example_stage = custom_target(
  'example-stage',
  input: example_targets,
  output: 'example-stage.stamp',
  build_by_default: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      stage_dir="$1"
      stamp="$2"
      shift 2
      mkdir -p "$stage_dir"
      for input in "$@"; do
        cp "$input" "$stage_dir/$(basename "$input")"
        chmod 0755 "$stage_dir/$(basename "$input")"
      done
      : > "$stamp"
    ''',
    'meson-stage-examples',
    meson.current_build_dir() / 'examples',
    '@OUTPUT@',
    '@INPUT@',
  ],
)

vips_gir = custom_target(
  'vips-gir',
  input: libvips_stage,
  output: 'Vips-8.0.gir',
  build_by_default: true,
  command: [
    g_ir_scanner,
    '--quiet',
    '--warn-all',
    '--namespace=Vips',
    '--nsversion=8.0',
    '--identifier-prefix=Vips',
    '--symbol-prefix=vips',
    '--output',
    '@OUTPUT@',
    '--no-libtool',
    '--library',
    'vips',
    '--library-path',
    runtime_libdir,
    '--include=GObject-2.0',
    '--pkg=glib-2.0',
    '--pkg=gio-2.0',
    '--pkg=gobject-2.0',
    '-I' + (meson.current_source_dir() / 'include'),
    meson.current_source_dir() / 'include' / 'vips' / 'vips.h',
  ],
  depends: [libvips_stage, libvips_cpp_stage, example_stage],
  env: environment({
    'LD_LIBRARY_PATH': runtime_libdir,
  }),
  install: true,
  install_dir: get_option('datadir') / 'gir-1.0',
)

vips_typelib = custom_target(
  'vips-typelib',
  input: vips_gir,
  output: 'Vips-8.0.typelib',
  build_by_default: true,
  command: [g_ir_compiler, '--output', '@OUTPUT@', '@INPUT@'],
  depends: [libvips_stage, vips_gir],
  install: true,
  install_dir: get_option('libdir') / 'girepository-1.0',
)

subdir('tools')

module_entrypoints = {
  'vips-heif': [
    'vips_foreign_load_heif_buffer_get_type',
    'vips_foreign_load_heif_file_get_type',
    'vips_foreign_load_heif_source_get_type',
    'vips_foreign_save_avif_target_get_type',
    'vips_foreign_save_heif_buffer_get_type',
    'vips_foreign_save_heif_file_get_type',
    'vips_foreign_save_heif_target_get_type',
  ],
  'vips-jxl': [
    'vips_foreign_load_jxl_buffer_get_type',
    'vips_foreign_load_jxl_file_get_type',
    'vips_foreign_load_jxl_source_get_type',
    'vips_foreign_save_jxl_buffer_get_type',
    'vips_foreign_save_jxl_file_get_type',
    'vips_foreign_save_jxl_target_get_type',
  ],
  'vips-magick': [
    'vips_foreign_load_magick7_buffer_get_type',
    'vips_foreign_load_magick7_file_get_type',
    'vips_foreign_load_magick_buffer_get_type',
    'vips_foreign_load_magick_file_get_type',
    'vips_foreign_save_magick_bmp_buffer_get_type',
    'vips_foreign_save_magick_bmp_file_get_type',
    'vips_foreign_save_magick_buffer_get_type',
    'vips_foreign_save_magick_file_get_type',
  ],
  'vips-openslide': [
    'vips_foreign_load_openslide_file_get_type',
    'vips_foreign_load_openslide_source_get_type',
  ],
  'vips-poppler': [
    'vips_foreign_load_pdf_buffer_get_type',
    'vips_foreign_load_pdf_file_get_type',
    'vips_foreign_load_pdf_source_get_type',
  ],
}

module_targets = []
foreach module_name, entrypoints : module_entrypoints
  module_deps = [glib_dep, gobject_dep, gmodule_dep]
  if dl_dep.found()
    module_deps += [dl_dep]
  endif

  module_wrapper_functions = ''
  module_registration_calls = ''
  foreach entrypoint : entrypoints
    module_wrapper_functions += '''
G_MODULE_EXPORT GType
@0@(void)
{
    return safe_vips_module_call("@0@");
}

'''.format(entrypoint)
    module_registration_calls += '''
    if (safe_vips_module_call("@0@") == 0)
        return "failed to register @0@";
'''.format(entrypoint)
  endforeach

  module_config = configuration_data()
  module_config.set('ENTRYPOINT_WRAPPERS', module_wrapper_functions)
  module_config.set('ENTRYPOINT_REGISTRATIONS', module_registration_calls)
  module_source = configure_file(
    input: 'build_support/module_wrapper.c.in',
    output: module_name + '-module.c',
    configuration: module_config,
  )

  module_targets += shared_module(
    module_name,
    module_source,
    name_prefix: '',
    build_by_default: true,
    install: true,
    install_dir: get_option('libdir') / module_dir_basename,
    dependencies: module_deps,
    link_args: [runtime_link],
    link_depends: libvips_stage,
    build_rpath: runtime_libdir,
  )
endforeach

module_stage = custom_target(
  'module-stage',
  input: module_targets,
  output: 'module-stage.stamp',
  build_by_default: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      stage_dir="$1"
      stamp="$2"
      shift 2
      mkdir -p "$stage_dir"
      for input in "$@"; do
        cp "$input" "$stage_dir/$(basename "$input")"
      done
      : > "$stamp"
    ''',
    'meson-module-stage',
    runtime_module_dir,
    '@OUTPUT@',
    '@INPUT@',
  ],
)

upstream_scripts_stage = custom_target(
  'upstream-scripts-stage',
  input: [
    original_root / 'test' / 'variables.sh.in',
    original_root / 'test' / 'test_cli.sh',
    original_root / 'test' / 'test_formats.sh',
    original_root / 'test' / 'test_keep.sh',
    original_root / 'test' / 'test_connections.sh',
    original_root / 'test' / 'test_descriptors.sh',
    original_root / 'test' / 'test_seq.sh',
    original_root / 'test' / 'test_stall.sh',
    original_root / 'test' / 'test_thumbnail.sh',
    original_root / 'test' / 'test_threading.sh',
    original_root / 'fuzz' / 'test_fuzz.sh',
  ],
  output: 'upstream-scripts-stage.stamp',
  build_by_default: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      build_root="$1"
      python_bin="$2"
      original_root="$3"
      stamp="$4"

      mkdir -p "$build_root/test" "$build_root/fuzz"

      sed -e "s|@abs_top_srcdir@|$original_root|g" -e "s|@abs_top_builddir@|$build_root|g" -e "s|@PYTHON@|$python_bin|g" "$original_root/test/variables.sh.in" > "$build_root/test/variables.sh"
      cat >> "$build_root/test/variables.sh" <<'EOF'

test_supported() {
	format=$1

	case "$format" in
		heifload|heifload_buffer|heifload_source|heifsave|heifsave_buffer|heifsave_target|avifsave|avifsave_target)
			echo "support for $format not configured, skipping test"
			return 1
			;;
	esac

	if $vips $format > /dev/null 2>&1; then
		result=0
	else
		echo "support for $format not configured, skipping test"
		result=1
	fi

	return $result
}
EOF
      chmod +x "$build_root/test/variables.sh"

      for script in test_cli.sh test_formats.sh test_keep.sh test_connections.sh test_descriptors.sh test_seq.sh test_stall.sh test_thumbnail.sh test_threading.sh
      do
        cp "$original_root/test/$script" "$build_root/test/$script"
        chmod +x "$build_root/test/$script"
      done

      cp "$original_root/fuzz/test_fuzz.sh" "$build_root/fuzz/test_fuzz.sh"
      chmod +x "$build_root/fuzz/test_fuzz.sh"

      : > "$stamp"
    ''',
    'meson-stage-upstream-scripts',
    meson.current_build_dir(),
    python3.full_path(),
    original_root,
    '@OUTPUT@',
  ],
)

upstream_test_env = environment()
upstream_test_env.prepend('LD_LIBRARY_PATH', runtime_libdir)
upstream_test_env.prepend('PYTHONPATH', vendor_pyvips_dir)
upstream_test_env.set('PYTHONNOUSERSITE', '1')
upstream_test_env.set('PIP_NO_INDEX', '1')
upstream_test_env.set('VIPSHOME', meson.current_build_dir())

standalone_engine = static_library(
  'standalone_engine',
  original_root / 'fuzz' / 'StandaloneFuzzTargetMain.c',
  build_by_default: true,
)

fuzz_execs = []
foreach fuzz_prog : [
  'gifsave_buffer_fuzzer',
  'jpegsave_buffer_fuzzer',
  'jpegsave_file_fuzzer',
  'mosaic_fuzzer',
  'pngsave_buffer_fuzzer',
  'sharpen_fuzzer',
  'smartcrop_fuzzer',
  'thumbnail_fuzzer',
  'webpsave_buffer_fuzzer',
]
  fuzz_execs += executable(
    fuzz_prog,
    original_root / 'fuzz' / (fuzz_prog + '.cc'),
    build_by_default: true,
    include_directories: test_include,
    dependencies: [glib_dep, gobject_dep, gio_dep],
    link_args: [runtime_link],
    link_depends: libvips_stage,
    link_with: standalone_engine,
    build_rpath: runtime_libdir,
  )
endforeach

fuzz_stage = custom_target(
  'fuzz-stage',
  input: fuzz_execs,
  output: 'fuzz-stage.stamp',
  build_by_default: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      stage_dir="$1"
      stamp="$2"
      shift 2
      mkdir -p "$stage_dir"
      for input in "$@"; do
        cp "$input" "$stage_dir/$(basename "$input")"
      done
      : > "$stamp"
    ''',
    'meson-fuzz-stage',
    meson.current_build_dir() / 'fuzz',
    '@OUTPUT@',
    '@INPUT@',
  ],
)

test_connections = executable(
  'test_connections',
  root_dir / 'original' / 'test' / 'test_connections.c',
  build_by_default: true,
  c_args: test_c_args,
  include_directories: test_include,
  dependencies: [glib_dep, gobject_dep, gio_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
  build_rpath: runtime_libdir,
)

test_descriptors = executable(
  'test_descriptors',
  root_dir / 'original' / 'test' / 'test_descriptors.c',
  build_by_default: true,
  c_args: test_c_args,
  include_directories: test_include,
  dependencies: [glib_dep, gobject_dep, gio_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
  build_rpath: runtime_libdir,
)

test_timeout_webpsave = executable(
  'test_timeout_webpsave',
  original_root / 'test' / 'test_timeout_webpsave.c',
  build_by_default: false,
  c_args: test_c_args,
  include_directories: test_include,
  dependencies: [glib_dep, gobject_dep, gio_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
  build_rpath: runtime_libdir,
)

upstream_test_binaries_stage = custom_target(
  'upstream-test-binaries-stage',
  input: [test_connections, test_descriptors],
  output: 'upstream-test-binaries-stage.stamp',
  build_by_default: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      stage_dir="$1"
      stamp="$2"
      shift 2
      mkdir -p "$stage_dir"
      for input in "$@"; do
        cp "$input" "$stage_dir/$(basename "$input")"
        chmod +x "$stage_dir/$(basename "$input")"
      done
      : > "$stamp"
    ''',
    'meson-stage-upstream-test-binaries',
    meson.current_build_dir() / 'test',
    '@OUTPUT@',
    '@INPUT@',
  ],
)

test(
  'connections',
  test_connections,
  args: [sample_png, meson.current_build_dir() / 'test_connections.png'],
  depends: libvips_stage,
)

test(
  'descriptors',
  test_descriptors,
  args: [sample_png],
  depends: libvips_stage,
)

test(
  'webpsave_timeout',
  test_timeout_webpsave,
  depends: libvips_stage,
  env: upstream_test_env,
)

foreach script_test : [
  'cli',
  'formats',
  'keep',
  'seq',
  'stall',
  'threading',
]
  test(
    script_test,
    sh,
    args: [meson.current_build_dir() / 'test' / ('test_' + script_test + '.sh')],
    workdir: meson.current_build_dir() / 'test',
    env: upstream_test_env,
    depends: [libvips_stage, module_stage, upstream_scripts_stage] + tool_targets,
    timeout: 120,
  )
endforeach

test(
  'fuzz',
  sh,
  args: ['-ec', 'cd "$1" && ./test_fuzz.sh', 'meson-fuzz-test', meson.current_build_dir() / 'fuzz'],
  env: upstream_test_env,
  depends: [libvips_stage, module_stage, upstream_scripts_stage, fuzz_stage, standalone_engine] + tool_targets,
  timeout: 120,
)

test(
  'python-suite',
  python3,
  args: ['-m', 'pytest', original_root / 'test' / 'test-suite'],
  workdir: meson.current_build_dir(),
  env: upstream_test_env,
  depends: [libvips_stage, module_stage] + tool_targets,
  timeout: 120,
)
