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
