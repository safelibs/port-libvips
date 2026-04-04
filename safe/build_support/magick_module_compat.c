#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include <vips/vips.h>

#if (defined(HAVE_MAGICK6) || defined(HAVE_MAGICK7)) && defined(MAGICK_MODULE)

extern GType vips_icc_import_get_type(void);
extern GType vips_icc_export_get_type(void);
extern GType vips_icc_transform_get_type(void);

#ifdef ENABLE_MAGICKSAVE
extern GType vips_foreign_save_magick_gif_file_get_type(void);
extern GType vips_foreign_save_magick_gif_buffer_get_type(void);
#endif

#if defined(HAVE_MAGICK6) && !defined(HAVE_MAGICK7)

typedef struct _VipsForeignLoadMagick7 {
	VipsObject parent_instance;
} VipsForeignLoadMagick7;

typedef VipsObjectClass VipsForeignLoadMagick7Class;

G_DEFINE_ABSTRACT_TYPE(VipsForeignLoadMagick7, vips_foreign_load_magick7,
	VIPS_TYPE_OBJECT);

static void
vips_foreign_load_magick7_class_init(VipsForeignLoadMagick7Class *class)
{
	VipsObjectClass *object_class = VIPS_OBJECT_CLASS(class);

	object_class->description = "ImageMagick v7 compatibility placeholder";
}

static void
vips_foreign_load_magick7_init(VipsForeignLoadMagick7 *self)
{
	(void) self;
}

typedef struct _VipsForeignLoadMagick7File {
	VipsForeignLoadMagick7 parent_instance;
} VipsForeignLoadMagick7File;

typedef VipsForeignLoadMagick7Class VipsForeignLoadMagick7FileClass;

G_DEFINE_TYPE(VipsForeignLoadMagick7File, vips_foreign_load_magick7_file,
	vips_foreign_load_magick7_get_type());

static void
vips_foreign_load_magick7_file_class_init(VipsForeignLoadMagick7FileClass *class)
{
	(void) class;
}

static void
vips_foreign_load_magick7_file_init(VipsForeignLoadMagick7File *self)
{
	(void) self;
}

typedef struct _VipsForeignLoadMagick7Buffer {
	VipsForeignLoadMagick7 parent_instance;
} VipsForeignLoadMagick7Buffer;

typedef VipsForeignLoadMagick7Class VipsForeignLoadMagick7BufferClass;

G_DEFINE_TYPE(VipsForeignLoadMagick7Buffer, vips_foreign_load_magick7_buffer,
	vips_foreign_load_magick7_get_type());

static void
vips_foreign_load_magick7_buffer_class_init(VipsForeignLoadMagick7BufferClass *class)
{
	(void) class;
}

static void
vips_foreign_load_magick7_buffer_init(VipsForeignLoadMagick7Buffer *self)
{
	(void) self;
}

#endif

__attribute__((constructor)) static void
safe_vips_magick_module_compat_register(void)
{
#ifdef ENABLE_MAGICKSAVE
	vips_foreign_save_magick_gif_file_get_type();
	vips_foreign_save_magick_gif_buffer_get_type();
#endif
#ifdef HAVE_LCMS2
	vips_icc_import_get_type();
	vips_icc_export_get_type();
	vips_icc_transform_get_type();
#endif
#if defined(HAVE_MAGICK6) && !defined(HAVE_MAGICK7)
	vips_foreign_load_magick7_get_type();
	vips_foreign_load_magick7_file_get_type();
	vips_foreign_load_magick7_buffer_get_type();
#endif
}

#endif
