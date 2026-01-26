project('mellow', 'rust', version: '0.1.0', meson_version: '>= 1.4')

gnome = import('gnome')

base_id = 'com.github.userwithaname.Mellow'

is_devel = get_option('profile') == 'devel'
if is_devel
  profile = 'Devel'
  application_id = '@0@.@1@'.format(base_id, profile)
else
  profile = ''
  application_id = base_id
endif

bindir = get_option('prefix') / get_option('bindir')
localedir = get_option('prefix') / get_option('localedir')
datadir = get_option('prefix') / get_option('datadir')
pkgdatadir = datadir / meson.project_name()

# Execute `meson.build` files in these directories
subdir('data')
subdir('src')

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
