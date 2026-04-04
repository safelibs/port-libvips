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
glib_dep = dependency('glib-2.0')
gobject_dep = dependency('gobject-2.0')
gio_dep = dependency('gio-2.0')

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
sample_png = root_dir / 'original' / 'test' / 'test-suite' / 'images' / 'sample.png'

test_c_args = ['-DHAVE_UNISTD_H=1']
test_include = include_directories('include')
tool_c_args = [
  '-DGETTEXT_PACKAGE="vips-safe"',
  '-DENABLE_DEPRECATED=0',
  '-DENABLE_MODULES=0',
  '-DHAVE_ORC=0',
]
tool_include = include_directories('include', '../original/libvips/include')

vips_cli = executable(
  'vips',
  root_dir / 'original' / 'tools' / 'vips.c',
  build_by_default: false,
  c_args: tool_c_args,
  include_directories: tool_include,
  dependencies: [glib_dep, gobject_dep, gio_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
)

test_connections = executable(
  'test_connections',
  root_dir / 'original' / 'test' / 'test_connections.c',
  build_by_default: false,
  c_args: test_c_args,
  include_directories: test_include,
  dependencies: [glib_dep, gobject_dep, gio_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
)

test_descriptors = executable(
  'test_descriptors',
  root_dir / 'original' / 'test' / 'test_descriptors.c',
  build_by_default: false,
  c_args: test_c_args,
  include_directories: test_include,
  dependencies: [glib_dep, gobject_dep, gio_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
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
