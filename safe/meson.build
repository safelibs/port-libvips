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

cargo = find_program('cargo')
sh = find_program('sh')
python3 = find_program('python3')
glib_dep = dependency('glib-2.0')
gobject_dep = dependency('gobject-2.0')
gio_dep = dependency('gio-2.0')
cpp = meson.get_compiler('cpp')

version_parts = meson.project_version().split('.')
version_major = version_parts[0]
version_minor = version_parts[1]
library_version = '42.17.1'
soname = 'libvips.so.42'
full_library_name = 'libvips.so.' + library_version
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

runtime_libdir = meson.current_build_dir() / 'lib'
runtime_link = runtime_libdir / 'libvips.so'
root_dir = meson.current_source_dir() / '..'
original_root = root_dir / 'original'
sample_png = root_dir / 'original' / 'test' / 'test-suite' / 'images' / 'sample.png'
vendor_pyvips_dir = meson.current_source_dir() / 'vendor' / 'pyvips-3.1.1'
runtime_module_dir = runtime_libdir / module_dir_basename

test_c_args = ['-DHAVE_UNISTD_H=1']
test_include = include_directories('include')
tool_c_args = [
  '-DGETTEXT_PACKAGE="vips-safe"',
  '-DENABLE_DEPRECATED=0',
  '-DENABLE_MODULES=0',
  '-DHAVE_ORC=0',
]
tool_include = include_directories('include', '../original/libvips/include')

subdir('tools')

module_stub_source = files('build_support/module_stub.c')
module_targets = []
foreach module_name : [
  'vips-heif',
  'vips-jxl',
  'vips-magick',
  'vips-openslide',
  'vips-poppler',
]
  module_targets += shared_module(
    module_name,
    module_stub_source,
    name_prefix: '',
    build_by_default: true,
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

      for script in test_cli.sh test_formats.sh test_keep.sh test_connections.sh test_descriptors.sh test_seq.sh test_stall.sh test_threading.sh
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
