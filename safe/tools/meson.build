vips_cli = executable(
  'vips',
  root_dir / 'original' / 'tools' / 'vips.c',
  build_by_default: true,
  install: true,
  c_args: tool_c_args,
  include_directories: tool_include,
  dependencies: [glib_dep, gobject_dep, gio_dep, gmodule_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
  build_rpath: runtime_libdir,
)

vipsedit_cli = executable(
  'vipsedit',
  root_dir / 'original' / 'tools' / 'vipsedit.c',
  build_by_default: true,
  install: true,
  c_args: tool_c_args,
  include_directories: tool_include,
  dependencies: [glib_dep, gobject_dep, gio_dep, gmodule_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
  build_rpath: runtime_libdir,
)

vipsthumbnail_cli = executable(
  'vipsthumbnail',
  root_dir / 'original' / 'tools' / 'vipsthumbnail.c',
  build_by_default: true,
  install: true,
  c_args: tool_c_args,
  include_directories: tool_include,
  dependencies: [glib_dep, gobject_dep, gio_dep, gmodule_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
  build_rpath: runtime_libdir,
)

vipsheader_cli = executable(
  'vipsheader',
  root_dir / 'original' / 'tools' / 'vipsheader.c',
  build_by_default: true,
  install: true,
  c_args: tool_c_args,
  include_directories: tool_include,
  dependencies: [glib_dep, gobject_dep, gio_dep, gmodule_dep],
  link_args: [runtime_link],
  link_depends: libvips_stage,
  build_rpath: runtime_libdir,
)

vipsprofile = configure_file(
  input: root_dir / 'original' / 'tools' / 'vipsprofile',
  output: 'vipsprofile',
  copy: true,
)

install_data(
  vipsprofile,
  install_dir: get_option('bindir'),
  install_mode: 'rwxr-xr-x',
)

tool_targets = [
  vips_cli,
  vipsedit_cli,
  vipsthumbnail_cli,
  vipsheader_cli,
]
