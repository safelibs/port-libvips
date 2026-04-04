#ifdef HAVE_CONFIG_H
#include <config.h>
#endif

#include <glib-object.h>
#include <vips/vips.h>

#if defined(HAVE_FFTW) && defined(MAGICK_MODULE)

#include "pfreqfilt.h"

extern GType vips_fwfft_get_type(void);
extern GType vips_invfft_get_type(void);

GType
vips_freqfilt_get_type(void)
{
	static GType type = 0;

	if (!type)
		type = g_type_from_name("VipsFreqfilt");

	return type;
}

int
vips__fftproc(VipsObject *context,
	VipsImage *in, VipsImage **out, VipsFftProcessFn fn)
{
	VipsImage **bands;
	VipsImage **fft;
	int b;

	bands = (VipsImage **) vips_object_local_array(context, in->Bands);
	fft = (VipsImage **) vips_object_local_array(context, in->Bands);

	if (in->Bands == 1)
		return fn(context, in, out);

	for (b = 0; b < in->Bands; b++)
		if (vips_extract_band(in, &bands[b], b, NULL) ||
			fn(context, bands[b], &fft[b]))
			return -1;

	if (vips_bandjoin(fft, out, in->Bands, NULL))
		return -1;

	return 0;
}

__attribute__((constructor)) static void
safe_vips_freqfilt_module_compat_register(void)
{
	if (!vips_freqfilt_get_type())
		return;

	vips_fwfft_get_type();
	vips_invfft_get_type();
}

#endif
