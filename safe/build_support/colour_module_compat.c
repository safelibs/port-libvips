#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include <glib-object.h>

#if defined(HAVE_LCMS2) && defined(MAGICK_MODULE)

GType
vips_colour_get_type(void)
{
	static GType type = 0;

	if (!type)
		type = g_type_from_name("VipsColour");

	return type;
}

GType
vips_colour_transform_get_type(void)
{
	static GType type = 0;

	if (!type)
		type = g_type_from_name("VipsColourTransform");

	return type;
}

GType
vips_colour_code_get_type(void)
{
	static GType type = 0;

	if (!type)
		type = g_type_from_name("VipsColourCode");

	return type;
}

GType
vips_colour_difference_get_type(void)
{
	static GType type = 0;

	if (!type)
		type = g_type_from_name("VipsColourDifference");

	return type;
}

#endif
