project('vips-safe', 'c', 'cpp',
  version: '8.15.1',
  meson_version: '>=0.61',
  default_options: [
    'c_std=gnu99',
    'cpp_std=c++11',
    'buildtype=release',
    'b_ndebug=if-release',
    'default_library=shared',
  ],
)

i18n = import('i18n')

cargo = find_program('cargo')
cc_bin = find_program('cc')
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

surface_mode = run_command(
  sh,
  '-ec',
  'printf %s "${VIPS_SAFE_EXPORT_SURFACE:-core-bootstrap}"',
  check: true,
).stdout().strip()
if surface_mode != 'core-bootstrap' and surface_mode != 'full'
  error('unsupported VIPS_SAFE_EXPORT_SURFACE=' + surface_mode)
endif
full_surface_enabled = surface_mode == 'full'

if full_surface_enabled and (not g_ir_scanner.found() or not g_ir_compiler.found())
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
root_dir = meson.current_source_dir() / '..'
original_root = root_dir / 'original'
build_check_root = root_dir / 'build-check'
build_check_install_root = root_dir / 'build-check-install'
reference_libvips_artifact = build_check_root / 'libvips' / 'libvips.so'
reference_libvips_cpp_artifact = build_check_root / 'cplusplus' / cxx_full_library_name

cargo_target_dir = meson.current_source_dir() / 'target'
cargo_artifact = cargo_target_dir / 'release' / 'libvips.so'
cargo_static_artifact = cargo_target_dir / 'release' / 'libvips.a'
core_export_map_artifact = meson.current_source_dir() / 'generated' / 'export.map'
full_export_map_artifact = meson.current_source_dir() / 'generated' / 'export-full.map'

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
  output: full_library_name,
  build_by_default: true,
  build_always_stale: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      out_real="$1"
      stage_dir="$2"
      stage_name="$3"
      stage_soname="$4"
      cargo_artifact="$5"
      cargo_static_artifact="$6"
      core_export_map="$7"
      full_export_map="$8"
      surface="$9"
      cc_bin="${10}"
      shift 10

      "$@"
      test -f "$cargo_static_artifact"

      case "$surface" in
        core-bootstrap)
          export_map="$core_export_map"
          ;;
        full)
          export_map="$full_export_map"
          ;;
        *)
          echo "unsupported VIPS_SAFE_EXPORT_SURFACE=$surface" >&2
          exit 1
          ;;
      esac
      test -f "$export_map"

      "$cc_bin" -shared -Wl,-soname,"$stage_soname" -Wl,--no-undefined "-Wl,--version-script=$export_map" -Wl,--whole-archive "$cargo_static_artifact" -Wl,--no-whole-archive -o "$cargo_artifact" -lgio-2.0 -lgobject-2.0 -lglib-2.0 -lgobject-2.0 -lglib-2.0 -lgobject-2.0 -lglib-2.0 -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
      chmod 0755 "$cargo_artifact"

      mkdir -p "$stage_dir"
      cp "$cargo_artifact" "$out_real"
      chmod 0755 "$out_real"
      cp "$cargo_artifact" "$stage_dir/$stage_name"
      chmod 0755 "$stage_dir/$stage_name"
      ln -sf "$stage_name" "$stage_dir/$stage_soname"
      ln -sf "$stage_soname" "$stage_dir/libvips.so"
    ''',
    'meson-stage-rust-libvips',
    '@OUTPUT@',
    meson.current_build_dir() / 'lib',
    full_library_name,
    soname,
    cargo_artifact,
    cargo_static_artifact,
    core_export_map_artifact,
    full_export_map_artifact,
    surface_mode,
    cc_bin,
  ] + cargo_build_cmd,
  env: cargo_env,
  install: true,
  install_dir: get_option('libdir'),
)

install_symlink(
  soname,
  install_dir: get_option('libdir'),
  pointing_to: full_library_name,
)

install_symlink(
  'libvips.so',
  install_dir: get_option('libdir'),
  pointing_to: soname,
)

install_subdir(
  build_check_install_root / 'include' / 'vips',
  install_dir: get_option('includedir'),
)

pkgconfig_cfg = configuration_data()
pkgconfig_cfg.set('PREFIX', get_option('prefix'))
pkgconfig_cfg.set('INCLUDEDIR', get_option('includedir'))
pkgconfig_cfg.set('LIBDIR', get_option('libdir'))
pkgconfig_cfg.set('VERSION', meson.project_version())

vips_pc = configure_file(
  input: 'build_support/vips.pc.in',
  output: 'vips.pc',
  configuration: pkgconfig_cfg,
)

install_data(
  vips_pc,
  install_dir: get_option('libdir') / 'pkgconfig',
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
runtime_module_dir = runtime_libdir / module_dir_basename

libvips_cpp_stage = custom_target(
  'libvips-cpp-stage',
  input: reference_libvips_cpp_artifact,
  output: [
    cxx_full_library_name,
    cxx_soname,
    'libvips-cpp.so',
  ],
  build_by_default: true,
  command: [
    sh,
    '-ec',
    '''
      set -eu
      input="$1"
      out_real="$2"
      out_soname="$3"
      out_link="$4"
      stage_dir="$5"
      stage_name="$6"
      stage_soname="$7"

      mkdir -p "$stage_dir"
      cp "$input" "$out_real"
      ln -sf "$(basename "$out_real")" "$out_soname"
      ln -sf "$(basename "$out_soname")" "$out_link"
      cp "$input" "$stage_dir/$stage_name"
      ln -sf "$stage_name" "$stage_dir/$stage_soname"
      ln -sf "$stage_soname" "$stage_dir/libvips-cpp.so"
    ''',
    'meson-stage-reference-libvips-cpp',
    '@INPUT@',
    '@OUTPUT0@',
    '@OUTPUT1@',
    '@OUTPUT2@',
    runtime_libdir,
    cxx_full_library_name,
    cxx_soname,
  ],
  install: true,
  install_dir: [
    get_option('libdir'),
    get_option('libdir'),
    get_option('libdir'),
  ],
)

vips_cpp_pc = configure_file(
  input: 'build_support/vips-cpp.pc.in',
  output: 'vips-cpp.pc',
  configuration: pkgconfig_cfg,
)

install_data(
  vips_cpp_pc,
  install_dir: get_option('libdir') / 'pkgconfig',
)

if full_surface_enabled
sample_png = root_dir / 'original' / 'test' / 'test-suite' / 'images' / 'sample.png'
vendor_pyvips_dir = meson.current_source_dir() / 'vendor' / 'pyvips-3.1.1'

test_c_args = ['-DHAVE_UNISTD_H=1']
test_include = include_directories('../build-check-install/include')
tool_c_args = [
  '-DGETTEXT_PACKAGE="@0@"'.format(gettext_domain),
  '-DENABLE_DEPRECATED=0',
  '-DENABLE_MODULES=1',
  '-DHAVE_ORC=0',
]
tool_include = include_directories('../build-check-install/include', '../original/libvips/include')
cpp_include = include_directories('../build-check-install/include', '../original/cplusplus/include')

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
    '-I' + (build_check_install_root / 'include'),
    build_check_install_root / 'include' / 'vips' / 'vips.h',
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

module_include = include_directories(
  '../build-check-install/include',
  '../original/libvips/colour',
  '../original/libvips/include',
  '../original/libvips/freqfilt',
  '../original/libvips/foreign',
)

module_cfg = configuration_data()
module_cfg.set_quoted('GETTEXT_PACKAGE', gettext_domain)
module_cfg.set('ENABLE_MODULES', 1)
module_cfg.set('HAVE_UNISTD_H', 1)
module_cfg.set('ENABLE_MAGICKLOAD', 1)
module_cfg.set('ENABLE_MAGICKSAVE', 1)

module_magick_dep = dependency(get_option('magick-package'), required: get_option('magick'))
if not module_magick_dep.found()
  module_magick_dep = dependency('ImageMagick', required: get_option('magick'))
endif
if module_magick_dep.found()
  if module_magick_dep.version().version_compare('>=7.0')
    module_cfg.set('HAVE_MAGICK7', 1)
  else
    module_cfg.set('HAVE_MAGICK6', 1)
    magick_include = '#include <magick/api.h>'
    if cc.has_member('struct _ImageInfo', 'number_scenes', prefix: magick_include, dependencies: module_magick_dep)
      module_cfg.set('HAVE_NUMBER_SCENES', 1)
    endif
    foreach func_name : [
      'InheritException',
      'AcquireExceptionInfo',
      'SetImageProperty',
      'SetImageExtent',
      'AcquireImage',
      'GetVirtualPixels',
      'ResetImageProfileIterator',
      'ResetImageAttributeIterator',
      'ResetImagePropertyIterator',
      'MagickCoreGenesis',
      'SetImageOption',
      'BlobToStringInfo',
      'OptimizePlusImageLayers',
      'OptimizeImageTransparency',
      'GetMagicInfo',
    ]
      if cc.has_function(func_name, prefix: magick_include, dependencies: module_magick_dep)
        module_cfg.set('HAVE_' + func_name.to_upper(), 1)
      endif
    endforeach
    if cc.compiles(magick_include + '\nColorspaceType colorspace = CMYColorspace;', name: 'module magick has CMYColorspace', dependencies: module_magick_dep)
      module_cfg.set('HAVE_CMYCOLORSPACE', 1)
    endif
    if cc.compiles(magick_include + '\nColorspaceType colorspace = HCLpColorspace;', name: 'module magick has HCLpColorspace', dependencies: module_magick_dep)
      module_cfg.set('HAVE_HCLPCOLORSPACE', 1)
    endif
    if cc.compiles(magick_include + '\nint main() {(void)GetImageMagick(NULL, 0, NULL);}', name: 'module magick GetImageMagick takes three arguments', dependencies: module_magick_dep)
      module_cfg.set('HAVE_GETIMAGEMAGICK3', 1)
    endif
  endif
endif

module_heif_dep = dependency('libheif', version: '>=1.4.0', required: get_option('heif'))
if module_heif_dep.found()
  module_cfg.set('HAVE_HEIF', 1)
  if cpp.has_function('heif_image_handle_get_raw_color_profile', prefix: '#include <libheif/heif.h>', dependencies: module_heif_dep)
    module_cfg.set('HAVE_HEIF_COLOR_PROFILE', 1)
  endif
  if cpp.has_function('heif_context_set_maximum_image_size_limit', prefix: '#include <libheif/heif.h>', dependencies: module_heif_dep)
    module_cfg.set('HAVE_HEIF_SET_MAX_IMAGE_SIZE_LIMIT', 1)
  endif
  if cpp.has_member('struct heif_decoding_options', 'convert_hdr_to_8bit', prefix: '#include <libheif/heif.h>', dependencies: module_heif_dep)
    module_cfg.set('HAVE_HEIF_DECODING_OPTIONS_CONVERT_HDR_TO_8BIT', 1)
  endif
  if module_heif_dep.version().version_compare('>=1.7.0')
    module_cfg.set('HAVE_HEIF_AVIF', 1)
  endif
  if module_heif_dep.version().version_compare('>=1.13.0')
    module_cfg.set('HAVE_HEIF_INIT', 1)
  endif
  if module_heif_dep.version().version_compare('>=1.17.0')
    module_cfg.set('HAVE_HEIF_ERROR_SUCCESS', 1)
  endif
endif

module_jxl_dep = dependency('libjxl', version: '>=0.6', required: get_option('jpeg-xl'))
module_jxl_threads_dep = dependency('libjxl_threads', version: '>=0.6', required: get_option('jpeg-xl'))
if module_jxl_dep.found() and module_jxl_threads_dep.found()
  module_cfg.set('HAVE_LIBJXL', 1)
  if module_jxl_dep.version().version_compare('>=0.7')
    module_cfg.set('HAVE_LIBJXL_0_7', 1)
  endif
  if module_jxl_dep.version().version_compare('>=0.9')
    module_cfg.set('HAVE_LIBJXL_0_9', 1)
  endif
endif

module_fftw_dep = dependency('fftw3', required: get_option('fftw'))
if module_fftw_dep.found()
  module_cfg.set('HAVE_FFTW', 1)
endif

module_lcms_dep = dependency('lcms2', required: get_option('lcms'))
if module_lcms_dep.found()
  module_cfg.set('HAVE_LCMS2', 1)
endif

module_openslide_dep = dependency('openslide', version: '>=3.3.0', required: get_option('openslide'))
if module_openslide_dep.found()
  module_cfg.set('HAVE_OPENSLIDE', 1)
  if module_openslide_dep.version().version_compare('>=3.4.0')
    module_cfg.set('HAVE_OPENSLIDE_3_4', 1)
  endif
  if cc.has_function('openslide_get_icc_profile_size', dependencies: module_openslide_dep)
    module_cfg.set('HAVE_OPENSLIDE_ICC', 1)
  endif
endif

module_cairo_dep = dependency('cairo', version: '>=1.2', required: get_option('poppler'))
module_poppler_dep = dependency('poppler-glib', version: '>=0.16.0', required: get_option('poppler'))
if module_poppler_dep.found() and module_cairo_dep.found()
  module_cfg.set('HAVE_POPPLER', 1)
endif

module_config_h = configure_file(
  output: 'config.h',
  configuration: module_cfg,
)

module_config_dep = declare_dependency(
  sources: module_config_h,
  include_directories: include_directories('.'),
  compile_args: ['-DHAVE_CONFIG_H=1'],
)

module_targets = []
if module_heif_dep.found()
  module_targets += shared_module(
    'vips-heif',
    [
      meson.current_source_dir() / 'build_support' / 'heif_module_compat.c',
      original_root / 'libvips' / 'module' / 'heif.c',
      original_root / 'libvips' / 'foreign' / 'heifload.c',
      original_root / 'libvips' / 'foreign' / 'heifsave.c',
    ],
    name_prefix: '',
    build_by_default: true,
    install: true,
    install_dir: get_option('libdir') / module_dir_basename,
    include_directories: module_include,
    dependencies: [glib_dep, gobject_dep, gmodule_dep, module_config_dep, module_heif_dep],
    c_args: ['-DHEIF_MODULE=1'],
    link_args: [runtime_link],
    link_depends: libvips_stage,
    build_rpath: runtime_libdir,
    install_rpath: '$ORIGIN/..',
  )
endif

if module_jxl_dep.found() and module_jxl_threads_dep.found()
  module_targets += shared_module(
    'vips-jxl',
    [
      original_root / 'libvips' / 'module' / 'jxl.c',
      original_root / 'libvips' / 'foreign' / 'jxlload.c',
      original_root / 'libvips' / 'foreign' / 'jxlsave.c',
    ],
    name_prefix: '',
    build_by_default: true,
    install: true,
    install_dir: get_option('libdir') / module_dir_basename,
    include_directories: module_include,
    dependencies: [glib_dep, gobject_dep, gmodule_dep, module_config_dep, module_jxl_dep, module_jxl_threads_dep],
    c_args: ['-DLIBJXL_MODULE=1'],
    link_args: [runtime_link],
    link_depends: libvips_stage,
    build_rpath: runtime_libdir,
    install_rpath: '$ORIGIN/..',
  )
endif

if module_magick_dep.found()
  module_targets += shared_module(
    'vips-magick',
    [
      meson.current_source_dir() / 'build_support' / 'colour_module_compat.c',
      meson.current_source_dir() / 'build_support' / 'freqfilt_module_compat.c',
      meson.current_source_dir() / 'build_support' / 'magick_module_compat.c',
      original_root / 'libvips' / 'colour' / 'icc_transform.c',
      original_root / 'libvips' / 'freqfilt' / 'fwfft.c',
      original_root / 'libvips' / 'freqfilt' / 'invfft.c',
      original_root / 'libvips' / 'module' / 'magick.c',
      original_root / 'libvips' / 'foreign' / 'magick.c',
      original_root / 'libvips' / 'foreign' / 'magick2vips.c',
      original_root / 'libvips' / 'foreign' / 'magick6load.c',
      original_root / 'libvips' / 'foreign' / 'magick7load.c',
      original_root / 'libvips' / 'foreign' / 'vips2magick.c',
    ],
    name_prefix: '',
    build_by_default: true,
    install: true,
    install_dir: get_option('libdir') / module_dir_basename,
    include_directories: module_include,
    dependencies: [glib_dep, gobject_dep, gmodule_dep, module_config_dep, module_magick_dep, module_fftw_dep, module_lcms_dep],
    c_args: ['-DMAGICK_MODULE=1'],
    link_args: [runtime_link],
    link_depends: libvips_stage,
    build_rpath: runtime_libdir,
    install_rpath: '$ORIGIN/..',
  )
endif

if module_openslide_dep.found()
  module_targets += shared_module(
    'vips-openslide',
    [
      original_root / 'libvips' / 'module' / 'openslide.c',
      original_root / 'libvips' / 'foreign' / 'openslideload.c',
    ],
    name_prefix: '',
    build_by_default: true,
    install: true,
    install_dir: get_option('libdir') / module_dir_basename,
    include_directories: module_include,
    dependencies: [glib_dep, gobject_dep, gmodule_dep, module_config_dep, module_openslide_dep],
    c_args: ['-DOPENSLIDE_MODULE=1'],
    link_args: [runtime_link],
    link_depends: libvips_stage,
    build_rpath: runtime_libdir,
    install_rpath: '$ORIGIN/..',
  )
endif

if module_poppler_dep.found() and module_cairo_dep.found()
  module_targets += shared_module(
    'vips-poppler',
    [
      original_root / 'libvips' / 'module' / 'poppler.c',
      original_root / 'libvips' / 'foreign' / 'popplerload.c',
    ],
    name_prefix: '',
    build_by_default: true,
    install: true,
    install_dir: get_option('libdir') / module_dir_basename,
    include_directories: module_include,
    dependencies: [glib_dep, gobject_dep, gmodule_dep, module_config_dep, module_poppler_dep, module_cairo_dep],
    c_args: ['-DPOPPLER_MODULE=1'],
    link_args: [runtime_link],
    link_depends: libvips_stage,
    build_rpath: runtime_libdir,
    install_rpath: '$ORIGIN/..',
  )
endif

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
endif

# The upstream compatibility surfaces look under $prefix/lib. Mirror the
# installed multiarch libraries and modules there for install-prefix checks
# without shipping the duplicate payloads in the Debian packages.
meson.add_install_script(
  sh,
  '-ec',
  '''
    set -eu
    prefix="$1"
    libdir_rel="$2"
    module_dir="$3"
    destdir="${DESTDIR:-}"
    install_root="$destdir$prefix"
    multiarch_libdir="$install_root/$libdir_rel"
    compat_libdir="$install_root/lib"
    compat_module_dir="$compat_libdir/$module_dir"

    mkdir -p "$compat_libdir"
    for library in "$multiarch_libdir"/libvips*.so*; do
      cp -a "$library" "$compat_libdir/$(basename "$library")"
    done

    rm -rf "$compat_module_dir"
    if [ -d "$multiarch_libdir/$module_dir" ]; then
      cp -a "$multiarch_libdir/$module_dir" "$compat_module_dir"
    fi
  ''',
  'meson-install-compat',
  get_option('prefix'),
  get_option('libdir'),
  module_dir_basename,
)

if full_surface_enabled
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
    meson.current_source_dir() / 'tests' / 'upstream' / 'test_thumbnail.sh',
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
      safe_thumbnail="$4"
      stamp="$5"

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

      cp "$safe_thumbnail" "$build_root/test/test_thumbnail.sh"
      chmod +x "$build_root/test/test_thumbnail.sh"

      cp "$original_root/fuzz/test_fuzz.sh" "$build_root/fuzz/test_fuzz.sh"
      chmod +x "$build_root/fuzz/test_fuzz.sh"

      : > "$stamp"
    ''',
    'meson-stage-upstream-scripts',
    meson.current_build_dir(),
    python3.full_path(),
    original_root,
    meson.current_source_dir() / 'tests' / 'upstream' / 'test_thumbnail.sh',
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
endif
