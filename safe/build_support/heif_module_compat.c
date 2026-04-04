#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include <glib/gi18n-lib.h>

#include <vips/vips.h>

#if defined(HAVE_HEIF) && defined(HEIF_MODULE) && defined(HAVE_HEIF_AVIF)

typedef struct _SafeVipsAvifSaveCompat {
	VipsOperation parent_instance;
} SafeVipsAvifSaveCompat;

typedef VipsOperationClass SafeVipsAvifSaveCompatClass;

G_DEFINE_TYPE(SafeVipsAvifSaveCompat, safe_vips_avifsave_compat,
	VIPS_TYPE_OPERATION);

static void
safe_vips_avifsave_compat_class_init(SafeVipsAvifSaveCompatClass *class)
{
	VipsObjectClass *object_class = VIPS_OBJECT_CLASS(class);
	VipsOperationClass *operation_class = VIPS_OPERATION_CLASS(class);

	object_class->nickname = "avifsave";
	object_class->description = _("save image in AVIF format");
	operation_class->flags |= VIPS_OPERATION_DEPRECATED;
}

static void
safe_vips_avifsave_compat_init(SafeVipsAvifSaveCompat *self)
{
	(void) self;
}

__attribute__((constructor)) static void
safe_vips_heif_module_compat_register(void)
{
	safe_vips_avifsave_compat_get_type();
}

#endif
