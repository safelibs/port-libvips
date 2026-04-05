#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <vips/vips.h>
#include <vips/vips7compat.h>
#include <vips/deprecated.h>
#include <vips/almostdeprecated.h>

static int
fail(const char *message)
{
    const char *detail = vips_error_buffer();
    if (detail && *detail) {
        fprintf(stderr, "deprecated_c_api_smoke: %s: %s\n", message, detail);
        vips_error_clear();
    }
    else {
        fprintf(stderr, "deprecated_c_api_smoke: %s\n", message);
    }
    return 1;
}

int
main(int argc, char **argv)
{
    const char *program = argc > 0 ? argv[0] : "deprecated_c_api_smoke";
    int deprecated_format = FMTUCHAR;
    int deprecated_bits = BBBYTE;
    VipsImage *source;
    VipsImage *cropped;
    VipsImage *copied;
    IMAGE_BOX box;
    double avg;
    char name[FILENAME_MAX];
    char mode[FILENAME_MAX];

    if (im_init_world(program) != 0)
        return fail("im_init_world() failed");

    if (deprecated_format != VIPS_FORMAT_UCHAR)
        return fail("FMTUCHAR does not map to VIPS_FORMAT_UCHAR");
    if (deprecated_bits != 8)
        return fail("BBBYTE is not 8 bits");

    source = im_open("deprecated-source", "t");
    if (!source)
        return fail("im_open(source) failed");

    cropped = im_open("deprecated-cropped", "t");
    if (!cropped)
        return fail("im_open(cropped) failed");

    copied = im_open("deprecated-copied", "t");
    if (!copied)
        return fail("im_open(copied) failed");

    if (im_black(source, 8, 6, 1) != 0)
        return fail("im_black() failed");

    box.xstart = 2;
    box.ystart = 1;
    box.xsize = 3;
    box.ysize = 4;
    box.chsel = -1;
    if (im_extract(source, cropped, &box) != 0)
        return fail("im_extract() failed");

    if (im_copy(cropped, copied) != 0)
        return fail("im_copy() failed");

    avg = -1.0;
    if (im_avg(copied, &avg) != 0)
        return fail("im_avg() failed");
    if (avg != 0.0)
        return fail("im_avg() did not preserve a black image");

    if (copied->Xsize != 3 || copied->Ysize != 4 || copied->Bands != 1)
        return fail("deprecated extract/copy pipeline returned unexpected dimensions");

    im_filename_split("/tmp/deprecated-smoke.v:jpeg:Q=90", name, mode);
    if (strcmp(name, "/tmp/deprecated-smoke.v") != 0)
        return fail("im_filename_split() returned the wrong filename");
    if (strcmp(mode, "jpeg:Q=90") != 0)
        return fail("im_filename_split() returned the wrong mode");

    if (im_close(copied) != 0)
        return fail("im_close(copied) failed");
    if (im_close(cropped) != 0)
        return fail("im_close(cropped) failed");
    if (im_close(source) != 0)
        return fail("im_close(source) failed");

    vips_shutdown();
    return 0;
}
