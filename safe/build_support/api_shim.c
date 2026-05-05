#include <ctype.h>
#include <errno.h>
#include <glib.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <gobject/gvaluecollector.h>
#include <vips/vips.h>
#include <vips/debug.h>
#include <vips/private.h>

#if defined(__GNUC__)
#define VIPS_PUBLIC __attribute__((visibility("default")))
#else
#define VIPS_PUBLIC
#endif

#ifndef va_copy
#define va_copy(d, s) ((d) = (s))
#endif

extern VipsImage *safe_vips_image_new_from_source_internal(
    VipsSource *source, const char *option_string, int access);
extern int safe_vips_image_write_to_target_internal(
    VipsImage *image, const char *suffix, VipsTarget *target);
extern int safe_vips_crop_internal(
    VipsImage *in, VipsImage **out, int left, int top, int width, int height);
extern int safe_vips_avg_internal(VipsImage *image, double *out);
extern int safe_vips_composite2_internal(
    VipsImage *base, VipsImage *overlay, VipsImage **out,
    int x, int y);
extern int safe_vips_object_mark_argument_assigned(
    VipsObject *object, const char *name, gboolean assigned);

static void
safe_vips_unref_images(VipsImage **images, size_t n)
{
    size_t i;

    for (i = 0; i < n; i++)
        if (images[i])
            g_object_unref(images[i]);
}

static int
safe_vips_pythagoras(VipsImage *in, VipsImage **out)
{
    VipsImage *t[9] = { 0 };
    double exponent[] = { 0.5 };
    int i;
    int result = -1;

    if (!in || !out)
        return -1;

    for (i = 0; i < in->Bands; i++)
        if (vips_extract_band(in, &t[i], i, NULL))
            goto done;

    for (i = 0; i < in->Bands; i++)
        if (vips_multiply(t[i], t[i], &t[i + in->Bands], NULL))
            goto done;

    if (vips_sum(&t[in->Bands], &t[2 * in->Bands], in->Bands, NULL) ||
        vips_math2_const(t[2 * in->Bands], out,
            VIPS_OPERATION_MATH2_POW, exponent, 1, NULL))
        goto done;

    result = 0;

done:
    safe_vips_unref_images(t, G_N_ELEMENTS(t));
    return result;
}

int
safe_vips_smartcrop_attention_internal(VipsImage *in,
    double hscale, double vscale, double sigma,
    gboolean premultiplied,
    int *attention_x, int *attention_y)
{
    static double skin_vector[] = { -0.78, -0.57, -0.44 };
    static double ones[] = { 1.0, 1.0, 1.0 };
    static double five[] = { 5.0 };
    static double zero[] = { 0.0 };
    static double minus_five[] = { -5.0 };
    static double hundred[] = { -100.0 };
    static double hundred_offset[] = { 100.0 };
    static double edge_matrix[] = {
        0.0, -1.0, 0.0,
        -1.0, 4.0, -1.0,
        0.0, -1.0, 0.0
    };

    VipsImage *t[24] = { 0 };
    VipsImage *working = in;
    double max;
    int x_pos;
    int y_pos;
    int result = -1;

    if (!in || !attention_x || !attention_y)
        return -1;

    if (vips_image_hasalpha(working) && !premultiplied) {
        if (vips_premultiply(working, &t[22], NULL))
            goto done;
        working = t[22];
    }

    if (vips_resize(working, &t[17], hscale,
            "vscale", vscale,
            NULL))
        goto done;

    t[21] = vips_image_new_matrix_from_array(3, 3,
        edge_matrix, G_N_ELEMENTS(edge_matrix));
    if (!t[21])
        goto done;

    if (vips_colourspace(t[17], &t[0], VIPS_INTERPRETATION_XYZ, NULL) ||
        vips_extract_band(t[0], &t[1], 0, "n", 3, NULL))
        goto done;

    if (vips_extract_band(t[1], &t[2], 1, NULL) ||
        vips_conv(t[2], &t[3], t[21],
            "precision", VIPS_PRECISION_INTEGER,
            NULL) ||
        vips_linear(t[3], &t[4], five, zero, 1, NULL) ||
        vips_abs(t[4], &t[14], NULL))
        goto done;

    if (safe_vips_pythagoras(t[1], &t[5]) ||
        vips_divide(t[1], t[5], &t[6], NULL) ||
        vips_linear(t[6], &t[7], ones, skin_vector, 3, NULL) ||
        safe_vips_pythagoras(t[7], &t[8]) ||
        vips_linear(t[8], &t[9], hundred, hundred_offset, 1, NULL) ||
        vips_linear(t[2], &t[23], ones, minus_five, 1, NULL) ||
        vips_sign(t[23], &t[10], NULL) ||
        vips_black(&t[11], 1, 1, NULL) ||
        vips_ifthenelse(t[10], t[9], t[11], &t[15], NULL))
        goto done;

    if (vips_colourspace(t[1], &t[12], VIPS_INTERPRETATION_LAB, NULL) ||
        vips_extract_band(t[12], &t[13], 1, NULL) ||
        vips_ifthenelse(t[10], t[13], t[11], &t[16], NULL))
        goto done;

    if (vips_sum(&t[14], &t[18], 3, NULL) ||
        vips_gaussblur(t[18], &t[19], sigma, NULL) ||
        vips_max(t[19], &max, "x", &x_pos, "y", &y_pos, NULL))
        goto done;

    *attention_x = x_pos / hscale;
    *attention_y = y_pos / vscale;
    result = 0;

done:
    safe_vips_unref_images(t, G_N_ELEMENTS(t));
    return result;
}

static void *
safe_vips_argument_is_required(VipsObject *object,
    GParamSpec *pspec,
    VipsArgumentClass *argument_class,
    VipsArgumentInstance *argument_instance,
    void *a, void *b)
{
    (void) object;
    (void) a;
    (void) b;

    if ((argument_class->flags & VIPS_ARGUMENT_REQUIRED) &&
        (argument_class->flags & VIPS_ARGUMENT_CONSTRUCT) &&
        (argument_class->flags & VIPS_ARGUMENT_INPUT) &&
        !argument_instance->assigned)
        return pspec;

    return NULL;
}

static GParamSpec *
safe_vips_find_required(VipsObject *object)
{
    return (GParamSpec *) vips_argument_map(object,
        safe_vips_argument_is_required, NULL, NULL);
}

static char *
safe_vips_find_unquoted_char(char *text, char needle)
{
    gboolean escaped = FALSE;
    char quote = '\0';
    char *p;

    for (p = text; *p; p++) {
        if (quote) {
            if (escaped)
                escaped = FALSE;
            else if (*p == '\\')
                escaped = TRUE;
            else if (*p == quote)
                quote = '\0';
            continue;
        }

        if (*p == '\'' || *p == '"') {
            quote = *p;
            continue;
        }

        if (*p == needle)
            return p;
    }

    return NULL;
}

static char *
safe_vips_next_segment(char **cursor)
{
    gboolean escaped = FALSE;
    char quote = '\0';
    char *segment;
    char *p;

    if (!cursor || !*cursor)
        return NULL;

    segment = *cursor;
    while (*segment && g_ascii_isspace(*segment))
        segment += 1;
    if (!*segment) {
        *cursor = segment;
        return NULL;
    }

    p = segment;
    while (*p) {
        if (quote) {
            if (escaped)
                escaped = FALSE;
            else if (*p == '\\')
                escaped = TRUE;
            else if (*p == quote)
                quote = '\0';
        }
        else if (*p == '\'' || *p == '"')
            quote = *p;
        else if (*p == ',')
            break;

        p += 1;
    }

    if (*p == ',') {
        *p = '\0';
        p += 1;
    }

    *cursor = p;
    g_strstrip(segment);

    return *segment ? segment : NULL;
}

static void
safe_vips_unquote(char *text)
{
    size_t len;

    if (!text)
        return;

    g_strstrip(text);
    len = strlen(text);
    if (len >= 2 &&
        ((text[0] == '"' && text[len - 1] == '"') ||
            (text[0] == '\'' && text[len - 1] == '\''))) {
        memmove(text, text + 1, len - 2);
        text[len - 2] = '\0';
    }
}

static int
safe_vips_object_set_from_string_internal(VipsObject *object, const char *string)
{
    VipsObjectClass *class;
    char *buffer;
    char *cursor;
    char *segment;

    if (!object || !string)
        return -1;

    class = VIPS_OBJECT_GET_CLASS(object);
    if (!class)
        return -1;

    buffer = g_strdup(string);
    g_strstrip(buffer);
    if (buffer[0] == '[') {
        size_t len = strlen(buffer);
        if (len > 0 && buffer[len - 1] == ']') {
            buffer[len - 1] = '\0';
            memmove(buffer, buffer + 1, len);
        }
    }

    cursor = buffer;
    while ((segment = safe_vips_next_segment(&cursor))) {
        GParamSpec *pspec = NULL;
        VipsArgumentClass *argument_class = NULL;
        VipsArgumentInstance *argument_instance = NULL;
        char *equals = safe_vips_find_unquoted_char(segment, '=');

        if (equals) {
            char *name;
            char *value;

            *equals = '\0';
            name = g_strstrip(segment);
            value = g_strstrip(equals + 1);
            safe_vips_unquote(value);

            if (vips_object_set_argument_from_string(object, name, value)) {
                g_free(buffer);
                return -1;
            }

            continue;
        }

        safe_vips_unquote(segment);
        if (g_object_class_find_property(G_OBJECT_GET_CLASS(object), segment) &&
            !vips_object_get_argument(object, segment,
                &pspec, &argument_class, &argument_instance) &&
            (argument_class->flags & VIPS_ARGUMENT_CONSTRUCT) &&
            (argument_class->flags & VIPS_ARGUMENT_INPUT) &&
            G_IS_PARAM_SPEC_BOOLEAN(pspec)) {
            if (!argument_instance->assigned)
                g_object_set(object, g_param_spec_get_name(pspec), TRUE, NULL);
        }
        else if ((pspec = safe_vips_find_required(object))) {
            if (vips_object_set_argument_from_string(object,
                    g_param_spec_get_name(pspec), segment)) {
                g_free(buffer);
                return -1;
            }
        }
        else {
            vips_error(class->nickname, "unable to set '%s'", segment);
            g_free(buffer);
            return -1;
        }
    }

    g_free(buffer);
    return 0;
}

static int
safe_vips_operation_set_valist_required(VipsOperation *operation, va_list ap)
{
    VIPS_ARGUMENT_FOR_ALL(operation,
        pspec, argument_class, argument_instance)
    {
        g_assert(argument_instance);

        if ((argument_class->flags & VIPS_ARGUMENT_REQUIRED) &&
            !(argument_class->flags & VIPS_ARGUMENT_DEPRECATED)) {
            VIPS_ARGUMENT_COLLECT_SET(pspec, argument_class, ap);

            g_object_set_property(G_OBJECT(operation),
                g_param_spec_get_name(pspec), &value);
            if (safe_vips_object_mark_argument_assigned(
                    VIPS_OBJECT(operation),
                    g_param_spec_get_name(pspec),
                    TRUE))
                return -1;

            VIPS_ARGUMENT_COLLECT_GET(pspec, argument_class, ap);
            VIPS_ARGUMENT_COLLECT_END
        }
    }
    VIPS_ARGUMENT_FOR_ALL_END

    return 0;
}

static int
safe_vips_operation_get_valist_required(VipsOperation *operation, va_list ap)
{
    VIPS_ARGUMENT_FOR_ALL(operation,
        pspec, argument_class, argument_instance)
    {
        if ((argument_class->flags & VIPS_ARGUMENT_REQUIRED)) {
            VIPS_ARGUMENT_COLLECT_SET(pspec, argument_class, ap);
            VIPS_ARGUMENT_COLLECT_GET(pspec, argument_class, ap);

            if (!(argument_class->flags & VIPS_ARGUMENT_OUTPUT) ||
                !argument_instance->assigned)
                continue;

            g_object_get(G_OBJECT(operation),
                g_param_spec_get_name(pspec), arg, NULL);

            VIPS_ARGUMENT_COLLECT_END
        }
    }
    VIPS_ARGUMENT_FOR_ALL_END

    return 0;
}

static int
safe_vips_operation_get_valist_optional(VipsOperation *operation, va_list ap)
{
    char *name;

    for (name = va_arg(ap, char *); name; name = va_arg(ap, char *)) {
        GParamSpec *pspec;
        VipsArgumentClass *argument_class;
        VipsArgumentInstance *argument_instance;

        if (vips_object_get_argument(VIPS_OBJECT(operation), name,
                &pspec, &argument_class, &argument_instance))
            return -1;

        VIPS_ARGUMENT_COLLECT_SET(pspec, argument_class, ap);
        VIPS_ARGUMENT_COLLECT_GET(pspec, argument_class, ap);

        if ((argument_class->flags & VIPS_ARGUMENT_OUTPUT) && arg) {
            g_object_get(G_OBJECT(operation),
                g_param_spec_get_name(pspec), arg, NULL);
        }

        VIPS_ARGUMENT_COLLECT_END
    }

    return 0;
}

static int
safe_vips_call_composite2(va_list required, va_list optional)
{
    VipsImage *base;
    VipsImage *overlay;
    VipsImage **out;
    va_list required_copy;
    va_list optional_copy;
    char *name;
    int mode;
    int x;
    int y;
    int result;

    va_copy(required_copy, required);
    base = va_arg(required_copy, VipsImage *);
    overlay = va_arg(required_copy, VipsImage *);
    out = va_arg(required_copy, VipsImage **);
    mode = va_arg(required_copy, int);
    va_end(required_copy);

    if (mode != VIPS_BLEND_MODE_OVER) {
        vips_error("composite2",
            "%s", "only OVER blend mode is supported");
        return -1;
    }

    x = 0;
    y = 0;
    va_copy(optional_copy, optional);
    for (name = va_arg(optional_copy, char *); name;
        name = va_arg(optional_copy, char *)) {
        if (strcmp(name, "x") == 0)
            x = va_arg(optional_copy, int);
        else if (strcmp(name, "y") == 0)
            y = va_arg(optional_copy, int);
        else {
            vips_error("composite2",
                "unknown optional argument '%s'", name);
            va_end(optional_copy);
            return -1;
        }
    }
    va_end(optional_copy);

    result = safe_vips_composite2_internal(base, overlay, out, x, y);
    return result;
}

static int
safe_vips_call_by_name(const char *operation_name,
    const char *option_string, va_list required, va_list optional)
{
    VipsOperation *operation;
    int result;

    if (strcmp(operation_name, "composite2") == 0)
        return safe_vips_call_composite2(required, optional);

    if (!(operation = vips_operation_new(operation_name)))
        return -1;

    if (option_string &&
        safe_vips_object_set_from_string_internal(VIPS_OBJECT(operation),
            option_string)) {
        vips_object_unref_outputs(VIPS_OBJECT(operation));
        g_object_unref(operation);
        return -1;
    }

    result = vips_call_required_optional(&operation, required, optional);
    if (result) {
        vips_object_unref_outputs(VIPS_OBJECT(operation));
        g_object_unref(operation);
        return -1;
    }

    g_object_unref(operation);
    return result;
}

typedef struct _SafeVipsCallOptionOutput {
    VipsArgumentInstance *argument_instance;
    char *value;
} SafeVipsCallOptionOutput;

static void *
safe_vips_call_find_pspec(VipsObject *object,
    GParamSpec *pspec,
    VipsArgumentClass *argument_class,
    VipsArgumentInstance *argument_instance,
    void *a, void *b)
{
    const char *name = (const char *) a;

    (void) object;
    (void) b;

    if (!(argument_class->flags & VIPS_ARGUMENT_REQUIRED) &&
        (argument_class->flags & VIPS_ARGUMENT_CONSTRUCT) &&
        !argument_instance->assigned)
        if ((strlen(name) == 1 &&
                g_param_spec_get_name(pspec)[0] == name[0]) ||
            strcmp(g_param_spec_get_name(pspec), name) == 0)
            return argument_instance;

    return NULL;
}

static int
safe_vips_call_option_output(VipsObject *object, SafeVipsCallOptionOutput *output)
{
    VipsArgumentInstance *argument_instance = output->argument_instance;
    GParamSpec *pspec = ((VipsArgument *) argument_instance)->pspec;

    if (!object->constructed)
        return 0;

    return vips_object_get_argument_to_string(object,
        g_param_spec_get_name(pspec), output->value);
}

static void
safe_vips_call_option_output_free(VipsObject *object, SafeVipsCallOptionOutput *output)
{
    (void) object;
    g_free(output->value);
    g_free(output);
}

static gboolean
safe_vips_call_options_set(const gchar *option_name, const gchar *value,
    gpointer data, GError **error)
{
    VipsOperation *operation = (VipsOperation *) data;
    const char *name;
    VipsArgumentInstance *argument_instance;
    VipsArgumentClass *argument_class;
    GParamSpec *pspec;

    for (name = option_name; *name == '-'; name++)
        ;

    argument_instance = (VipsArgumentInstance *) vips_argument_map(
        VIPS_OBJECT(operation),
        safe_vips_call_find_pspec, (void *) name, NULL);

    if (!argument_instance) {
        vips_error(VIPS_OBJECT_GET_CLASS(operation)->nickname,
            "unknown argument '%s'", name);
        vips_error_g(error);
        return FALSE;
    }

    argument_class = argument_instance->argument_class;
    pspec = ((VipsArgument *) argument_instance)->pspec;

    if ((argument_class->flags & VIPS_ARGUMENT_INPUT)) {
        if (vips_object_set_argument_from_string(
                VIPS_OBJECT(operation),
                g_param_spec_get_name(pspec), value)) {
            vips_error_g(error);
            return FALSE;
        }
    }
    else if ((argument_class->flags & VIPS_ARGUMENT_OUTPUT)) {
        SafeVipsCallOptionOutput *output;

        output = g_new(SafeVipsCallOptionOutput, 1);
        output->argument_instance = argument_instance;
        output->value = g_strdup(value);
        g_signal_connect(operation, "postbuild",
            G_CALLBACK(safe_vips_call_option_output),
            output);
        g_signal_connect(operation, "close",
            G_CALLBACK(safe_vips_call_option_output_free),
            output);
    }

    return TRUE;
}

static void *
safe_vips_call_options_add(VipsObject *object,
    GParamSpec *pspec,
    VipsArgumentClass *argument_class,
    VipsArgumentInstance *argument_instance,
    void *a, void *b)
{
    GOptionGroup *group = (GOptionGroup *) a;
    (void) b;

    if (!(argument_class->flags & VIPS_ARGUMENT_REQUIRED) &&
        (argument_class->flags & VIPS_ARGUMENT_CONSTRUCT) &&
        !argument_instance->assigned) {
        const char *name = g_param_spec_get_name(pspec);
        gboolean needs_string =
            vips_object_argument_needsstring(object, name);
        GOptionEntry entry[2];

        entry[0].long_name = name;
        entry[0].description = g_param_spec_get_blurb(pspec);
        if (argument_class->flags & VIPS_ARGUMENT_DEPRECATED)
            entry[0].short_name = '\0';
        else
            entry[0].short_name = name[0];

        entry[0].flags = 0;
        if (!needs_string)
            entry[0].flags |= G_OPTION_FLAG_NO_ARG;
        if (argument_class->flags & VIPS_ARGUMENT_DEPRECATED)
            entry[0].flags |= G_OPTION_FLAG_HIDDEN;

        entry[0].arg = G_OPTION_ARG_CALLBACK;
        entry[0].arg_data = (gpointer) safe_vips_call_options_set;
        if (needs_string)
            entry[0].arg_description =
                g_type_name(G_PARAM_SPEC_VALUE_TYPE(pspec));
        else
            entry[0].arg_description = NULL;

        entry[1].long_name = NULL;
        g_option_group_add_entries(group, &entry[0]);
    }

    return NULL;
}

VIPS_PUBLIC VipsImage *
vips_image_new_from_source(VipsSource *source, const char *option_string, ...)
{
    va_list ap;
    const char *name;
    int access = VIPS_ACCESS_RANDOM;

    va_start(ap, option_string);
    while ((name = va_arg(ap, const char *))) {
        if (strcmp(name, "access") == 0)
            access = va_arg(ap, int);
        else
            (void) va_arg(ap, void *);
    }
    va_end(ap);

    return safe_vips_image_new_from_source_internal(source, option_string, access);
}

VIPS_PUBLIC VipsImage *
vips_image_new_from_file(const char *name, ...)
{
    char *filename;
    char *option_string;
    const char *operation_name;
    va_list ap;
    int result;
    VipsImage *out = NULL;

    if (!(filename = vips_filename_get_filename(name)))
        return NULL;
    if (!(option_string = vips_filename_get_options(name))) {
        g_free(filename);
        return NULL;
    }
    if (!(operation_name = vips_foreign_find_load(filename))) {
        g_free(option_string);
        g_free(filename);
        return NULL;
    }

    va_start(ap, name);
    result = vips_call_split_option_string(operation_name,
        option_string, ap, filename, &out);
    va_end(ap);

    g_free(option_string);
    g_free(filename);

    return result ? NULL : out;
}

VIPS_PUBLIC VipsImage *
vips_image_new_from_buffer(const void *buf, size_t len,
    const char *option_string, ...)
{
    const char *operation_name;
    va_list ap;
    int result;
    VipsImage *out = NULL;
    VipsBlob *blob;

    if (!(operation_name = vips_foreign_find_load_buffer(buf, len)))
        return NULL;

    blob = vips_blob_new(NULL, buf, len);
    va_start(ap, option_string);
    result = vips_call_split_option_string(operation_name,
        option_string, ap, blob, &out);
    va_end(ap);
    vips_area_unref(VIPS_AREA(blob));

    return result ? NULL : out;
}

VIPS_PUBLIC VipsImage *
vips_image_new_temp_file(const char *format)
{
    char *name;
    VipsImage *image;

    if (!(name = vips__temp_name(format ? format : "%s.v")))
        return NULL;
    if (!(image = vips_image_new_memory())) {
        g_free(name);
        return NULL;
    }

    g_object_set(image,
        "filename", name,
        "mode", "w",
        NULL);
    vips_image_set_delete_on_close(image, TRUE);
    g_free(name);

    return image;
}

VIPS_PUBLIC int
vips_image_write_to_target(VipsImage *in, const char *suffix, VipsTarget *target, ...)
{
    char *filename;
    char *option_string;
    const char *operation_name;
    va_list ap;
    int result;

    if (!(filename = vips_filename_get_filename(suffix)))
        return -1;
    if (!(option_string = vips_filename_get_options(suffix))) {
        g_free(filename);
        return -1;
    }

    operation_name = vips_foreign_find_save_target(filename);
    if (!operation_name) {
        g_free(option_string);
        g_free(filename);
        return safe_vips_image_write_to_target_internal(in, suffix, target);
    }

    va_start(ap, target);
    result = vips_call_split_option_string(operation_name,
        option_string, ap, in, target);
    va_end(ap);

    g_free(option_string);
    g_free(filename);

    return result;
}

VIPS_PUBLIC int
vips_image_write_to_file(VipsImage *image, const char *name, ...)
{
    char *filename;
    char *option_string;
    const char *operation_name;
    va_list ap;
    int result;

    if (!(filename = vips_filename_get_filename(name)))
        return -1;
    if (!(option_string = vips_filename_get_options(name))) {
        g_free(filename);
        return -1;
    }

    if ((operation_name = vips_foreign_find_save(filename))) {
        va_start(ap, name);
        result = vips_call_split_option_string(operation_name,
            option_string, ap, image, filename);
        va_end(ap);
    }
    else if ((operation_name = vips_foreign_find_save_target(filename))) {
        VipsTarget *target;

        if (!(target = vips_target_new_to_file(filename))) {
            g_free(option_string);
            g_free(filename);
            return -1;
        }

        va_start(ap, name);
        result = vips_call_split_option_string(operation_name,
            option_string, ap, image, target);
        va_end(ap);

        g_object_unref(target);
    }
    else
        result = -1;

    g_free(option_string);
    g_free(filename);

    return result;
}

VIPS_PUBLIC int
vips_image_write_to_buffer(VipsImage *in,
    const char *suffix, void **buf, size_t *len,
    ...)
{
    char *filename;
    char *option_string;
    const char *operation_name;
    va_list ap;
    int result;
    VipsBlob *blob = NULL;

    if (buf)
        *buf = NULL;
    if (len)
        *len = 0;

    if (!(filename = vips_filename_get_filename(suffix)))
        return -1;
    if (!(option_string = vips_filename_get_options(suffix))) {
        g_free(filename);
        return -1;
    }

    operation_name = vips_foreign_find_save_target(filename);
    if (operation_name) {
        VipsTarget *target;

        if (!(target = vips_target_new_to_memory())) {
            g_free(option_string);
            g_free(filename);
            return -1;
        }

        va_start(ap, len);
        result = vips_call_split_option_string(operation_name,
            option_string, ap, in, target);
        va_end(ap);

        if (!result)
            g_object_get(target, "blob", &blob, NULL);
        g_object_unref(target);
    }
    else if ((operation_name = vips_foreign_find_save_buffer(filename))) {
        va_start(ap, len);
        result = vips_call_split_option_string(operation_name,
            option_string, ap, in, &blob);
        va_end(ap);
    }
    else
        result = -1;

    g_free(option_string);
    g_free(filename);

    if (result)
        return -1;

    if (blob) {
        if (buf) {
            *buf = VIPS_AREA(blob)->data;
            VIPS_AREA(blob)->free_fn = NULL;
        }
        if (len)
            *len = VIPS_AREA(blob)->length;
        vips_area_unref(VIPS_AREA(blob));
    }

    return 0;
}

VIPS_PUBLIC int
vips_crop(VipsImage *in, VipsImage **out, int left, int top, int width, int height, ...)
{
    return safe_vips_crop_internal(in, out, left, top, width, height);
}

VIPS_PUBLIC int
vips_avg(VipsImage *in, double *out, ...)
{
    return safe_vips_avg_internal(in, out);
}

VIPS_PUBLIC char *
vips_strncpy(char *dest, const char *src, int n)
{
    if (!dest || n <= 0)
        return dest;
    if (!src) {
        dest[0] = '\0';
        return dest;
    }

    g_strlcpy(dest, src, (gsize) n);
    return dest;
}

VIPS_PUBLIC int
vips__substitute(char *buf, size_t len, char *sub)
{
    size_t buflen;
    size_t sublen;
    char *sub_start = NULL;
    char *sub_end = NULL;
    char *p;
    int lowest_n = -1;

    if (!buf || !sub || len == 0)
        return -1;

    buflen = strlen(buf);
    sublen = strlen(sub);
    if (buflen >= len)
        return -1;

    for (p = buf; (p = strchr(p, '%')); p++) {
        if (isdigit((unsigned char) p[1])) {
            char *q;

            for (q = p + 1; isdigit((unsigned char) *q); q++)
                ;
            if (*q == 's') {
                int n = atoi(p + 1);

                if (lowest_n == -1 || n < lowest_n) {
                    lowest_n = n;
                    sub_start = p;
                    sub_end = q + 1;
                }
            }
        }
    }

    if (!sub_start)
        for (p = buf; (p = strchr(p, '%')); p++)
            if (p[1] == 's') {
                sub_start = p;
                sub_end = p + 2;
                break;
            }

    if (!sub_start)
        return -1;

    {
        size_t before_len = (size_t) (sub_start - buf);
        size_t marker_len = (size_t) (sub_end - sub_start);
        size_t after_len = buflen - (before_len + marker_len);
        size_t final_len = before_len + sublen + after_len + 1;

        if (final_len > len)
            return -1;

        memmove(buf + before_len + sublen,
            buf + before_len + marker_len,
            after_len + 1);
        memmove(buf + before_len, sub, sublen);
    }

    return 0;
}

VIPS_PUBLIC int
vips_enum_from_nick(const char *domain, GType type, const char *nick)
{
    GEnumClass *genum;
    GEnumValue *value;

    if (!nick)
        return -1;
    if (!(genum = G_ENUM_CLASS(g_type_class_ref(type)))) {
        vips_error(domain, "%s", "no such enum type");
        return -1;
    }

    value = g_enum_get_value_by_name(genum, nick);
    if (!value)
        value = g_enum_get_value_by_nick(genum, nick);
    if (!value) {
        vips_error(domain, "enum '%s' has no member '%s'",
            g_type_name(type), nick);
        return -1;
    }

    return value->value;
}

VIPS_PUBLIC const char *
vips_enum_nick(GType type, int value)
{
    GEnumClass *genum;
    GEnumValue *entry;
    const char *nick = NULL;

    if (!(genum = G_ENUM_CLASS(g_type_class_ref(type))))
        return NULL;

    entry = g_enum_get_value(genum, value);
    if (entry)
        nick = entry->value_nick;

    g_type_class_unref(genum);

    return nick;
}

VIPS_PUBLIC int
vips_flags_from_nick(const char *domain, GType type, const char *nick)
{
    GFlagsClass *gflags;
    GFlagsValue *value;
    int result = 0;
    char **parts;
    int i;

    if (!nick)
        return -1;
    if (sscanf(nick, "%d", &result) == 1)
        return result;
    if (!(gflags = G_FLAGS_CLASS(g_type_class_ref(type)))) {
        vips_error(domain, "%s", "no such flag type");
        return -1;
    }

    parts = g_strsplit_set(nick, "\t;:|, ", -1);
    for (i = 0; parts[i]; i++) {
        if (parts[i][0] == '\0')
            continue;
        value = g_flags_get_value_by_name(gflags, parts[i]);
        if (!value)
            value = g_flags_get_value_by_nick(gflags, parts[i]);
        if (!value) {
            vips_error(domain, "flags '%s' has no member '%s'",
                g_type_name(type), parts[i]);
            g_strfreev(parts);
            return -1;
        }
        result |= value->value;
    }
    g_strfreev(parts);

    return result;
}

static gint64
safe_vips_image_pixel_length(VipsImage *image)
{
    gint64 psize;

    switch (image->Coding) {
    case VIPS_CODING_LABQ:
    case VIPS_CODING_RAD:
    case VIPS_CODING_NONE:
        psize = VIPS_IMAGE_SIZEOF_IMAGE(image);
        break;

    default:
        psize = image->Length;
        break;
    }

    return psize + image->sizeof_header;
}

static int
safe_vips_ftruncate(int fd, gint64 pos)
{
    if (ftruncate(fd, pos)) {
        vips_error_system(errno, "vips__write_extension_block",
            "%s", "unable to truncate");
        return -1;
    }

    return 0;
}

VIPS_PUBLIC int
vips__has_extension_block(VipsImage *im)
{
    if (!im || im->file_length <= 0)
        return 0;

    return im->file_length > safe_vips_image_pixel_length(im);
}

VIPS_PUBLIC void *
vips__read_extension_block(VipsImage *im, int *size)
{
    gint64 psize;
    gint64 extra;
    char *buf;

    if (size)
        *size = 0;

    if (!im || im->fd < 0 || im->file_length <= 0)
        return NULL;

    psize = safe_vips_image_pixel_length(im);
    extra = im->file_length - psize;
    if (extra <= 0)
        return NULL;
    if (extra > 100 * 1024 * 1024) {
        vips_error("VipsImage",
            "%s", "more than 100 megabytes of extension data?");
        return NULL;
    }
    if (vips__seek(im->fd, psize, SEEK_SET) == -1)
        return NULL;

    buf = g_malloc((size_t) extra + 1);
    if (read(im->fd, buf, (size_t) extra) != (ssize_t) extra) {
        g_free(buf);
        vips_error("VipsImage", "%s", "unable to read extension block");
        return NULL;
    }
    buf[extra] = '\0';

    if (size)
        *size = (int) extra;

    return buf;
}

VIPS_PUBLIC int
vips_linear(VipsImage *in, VipsImage **out,
    const double *a, const double *b, int n, ...)
{
    va_list ap;
    VipsArea *area_a;
    VipsArea *area_b;
    int result;

    area_a = VIPS_AREA(vips_array_double_new(a, n));
    area_b = VIPS_AREA(vips_array_double_new(b, n));

    va_start(ap, n);
    result = vips_call_split("linear", ap, in, out, area_a, area_b);
    va_end(ap);

    vips_area_unref(area_a);
    vips_area_unref(area_b);

    return result;
}

VIPS_PUBLIC int
vips_bandjoin(VipsImage **in, VipsImage **out, int n, ...)
{
    va_list ap;
    VipsArrayImage *array;
    int result;

    array = vips_array_image_new(in, n);

    va_start(ap, n);
    result = vips_call_split("bandjoin", ap, array, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(array));

    return result;
}

VIPS_PUBLIC int
vips_bandjoin_const(VipsImage *in, VipsImage **out, double *c, int n, ...)
{
    va_list ap;
    VipsArrayDouble *array;
    int result;

    array = vips_array_double_new(c, n);

    va_start(ap, n);
    result = vips_call_split("bandjoin_const", ap, in, out, array);
    va_end(ap);

    vips_area_unref(VIPS_AREA(array));

    return result;
}

VIPS_PUBLIC int
vips_sum(VipsImage **in, VipsImage **out, int n, ...)
{
    va_list ap;
    VipsArrayImage *array;
    int result;

    array = vips_array_image_new(in, n);

    va_start(ap, n);
    result = vips_call_split("sum", ap, array, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(array));

    return result;
}

VIPS_PUBLIC int
vips_arrayjoin(VipsImage **in, VipsImage **out, int n, ...)
{
    va_list ap;
    VipsArrayImage *array;
    int result;

    array = vips_array_image_new(in, n);

    va_start(ap, n);
    result = vips_call_split("arrayjoin", ap, array, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(array));

    return result;
}

VIPS_PUBLIC int
vips_bandrank(VipsImage **in, VipsImage **out, int n, ...)
{
    va_list ap;
    VipsArrayImage *array;
    int result;

    array = vips_array_image_new(in, n);

    va_start(ap, n);
    result = vips_call_split("bandrank", ap, array, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(array));

    return result;
}

VIPS_PUBLIC int
vips_case(VipsImage *index, VipsImage **cases, VipsImage **out, int n, ...)
{
    va_list ap;
    VipsArrayImage *array;
    int result;

    array = vips_array_image_new(cases, n);

    va_start(ap, n);
    result = vips_call_split("case", ap, index, array, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(array));

    return result;
}

VIPS_PUBLIC int
vips_switch(VipsImage **tests, VipsImage **out, int n, ...)
{
    va_list ap;
    VipsArrayImage *tests_array;
    int result;

    tests_array = vips_array_image_new(tests, n);

    va_start(ap, n);
    result = vips_call_split("switch", ap, tests_array, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(tests_array));

    return result;
}

VIPS_PUBLIC int
vips_affine(VipsImage *in, VipsImage **out,
    double a, double b, double c, double d, ...)
{
    va_list ap;
    VipsArea *matrix;
    double values[4] = { a, b, c, d };
    int result;

    matrix = VIPS_AREA(vips_array_double_new(values, 4));

    va_start(ap, d);
    result = vips_call_split("affine", ap, in, out, matrix);
    va_end(ap);

    vips_area_unref(matrix);

    return result;
}

VIPS_PUBLIC int
vips_getpoint(VipsImage *in, double **vector, int *n, int x, int y, ...)
{
    va_list ap;
    VipsArrayDouble *out_array = NULL;
    VipsArea *area;
    int result;

    va_start(ap, y);
    result = vips_call_split("getpoint", ap, in, &out_array, x, y);
    va_end(ap);

    if (result)
        return -1;

    area = VIPS_AREA(out_array);
    *vector = VIPS_ARRAY(NULL, area->n, double);
    if (!*vector) {
        vips_area_unref(area);
        return -1;
    }

    memcpy(*vector, area->data, area->n * area->sizeof_type);
    *n = area->n;
    vips_area_unref(area);

    return 0;
}

VIPS_PUBLIC int
vips_pngload_buffer(void *buf, size_t len, VipsImage **out, ...)
{
    va_list ap;
    VipsBlob *blob;
    int result;

    blob = vips_blob_new(NULL, buf, len);

    va_start(ap, out);
    result = vips_call_split("pngload_buffer", ap, blob, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(blob));

    return result;
}

VIPS_PUBLIC int
vips_jpegload_buffer(void *buf, size_t len, VipsImage **out, ...)
{
    va_list ap;
    VipsBlob *blob;
    int result;

    blob = vips_blob_new(NULL, buf, len);

    va_start(ap, out);
    result = vips_call_split("jpegload_buffer", ap, blob, out);
    va_end(ap);

    vips_area_unref(VIPS_AREA(blob));

    return result;
}

VIPS_PUBLIC int
vips_pngsave_buffer(VipsImage *in, void **buf, size_t *len, ...)
{
    va_list ap;
    VipsArea *area;
    int result;

    area = NULL;

    va_start(ap, len);
    result = vips_call_split("pngsave_buffer", ap, in, &area);
    va_end(ap);

    if (!result &&
        area) {
        if (buf) {
            *buf = area->data;
            area->free_fn = NULL;
        }
        if (len)
            *len = area->length;

        vips_area_unref(area);
    }

    return result;
}

VIPS_PUBLIC int
vips_jpegsave_buffer(VipsImage *in, void **buf, size_t *len, ...)
{
    va_list ap;
    VipsArea *area;
    int result;

    area = NULL;

    va_start(ap, len);
    result = vips_call_split("jpegsave_buffer", ap, in, &area);
    va_end(ap);

    if (!result &&
        area) {
        if (buf) {
            *buf = area->data;
            area->free_fn = NULL;
        }
        if (len)
            *len = area->length;

        vips_area_unref(area);
    }

    return result;
}

VIPS_PUBLIC int
vips_gifsave_buffer(VipsImage *in, void **buf, size_t *len, ...)
{
    return vips_pngsave_buffer(in, buf, len, NULL);
}

VIPS_PUBLIC int
vips_rot180(VipsImage *in, VipsImage **out, ...)
{
    return vips_rot(in, out, VIPS_ANGLE_D180, NULL);
}

VIPS_PUBLIC void
vips_g_error(GError **error)
{
    if (error &&
        *error) {
        vips_error("glib", "%s\n", (*error)->message);
        g_error_free(*error);
        *error = NULL;
    }
}

VIPS_PUBLIC char *
vips__file_read(FILE *fp, const char *filename, size_t *length_out)
{
    size_t capacity;
    size_t length;
    char *buffer;

    if (!fp) {
        vips_error("vips__file_read", "%s", "file handle is null");
        return NULL;
    }

    if (fseek(fp, 0, SEEK_END) == 0) {
        long end = ftell(fp);

        if (end >= 0) {
            if ((unsigned long) end > 1024UL * 1024UL * 1024UL) {
                vips_error("vips__file_read", "\"%s\" too long",
                    filename ? filename : "stream");
                return NULL;
            }
            capacity = (size_t) end + 1;
            if (fseek(fp, 0, SEEK_SET) != 0)
                return NULL;
            buffer = g_try_malloc(capacity);
            if (!buffer) {
                vips_error("vips__file_read", "%s", "out of memory");
                return NULL;
            }
            length = fread(buffer, 1, (size_t) end, fp);
            if (length != (size_t) end) {
                g_free(buffer);
                vips_error("vips__file_read", "error reading from file \"%s\"",
                    filename ? filename : "stream");
                return NULL;
            }
            buffer[length] = '\0';
            if (length_out)
                *length_out = length;
            return buffer;
        }

        rewind(fp);
    }

    capacity = 4096;
    length = 0;
    buffer = g_try_malloc(capacity);
    if (!buffer) {
        vips_error("vips__file_read", "%s", "out of memory");
        return NULL;
    }

    for (;;) {
        size_t remaining = capacity - length - 1;
        size_t chunk;

        if (remaining == 0) {
            char *grown;

            if (capacity >= 1024UL * 1024UL * 1024UL) {
                g_free(buffer);
                vips_error("vips__file_read", "\"%s\" too long",
                    filename ? filename : "stream");
                return NULL;
            }

            capacity *= 2;
            grown = g_try_realloc(buffer, capacity);
            if (!grown) {
                g_free(buffer);
                vips_error("vips__file_read", "%s", "out of memory");
                return NULL;
            }
            buffer = grown;
            remaining = capacity - length - 1;
        }

        chunk = fread(buffer + length, 1, remaining, fp);
        length += chunk;
        if (chunk < remaining) {
            if (ferror(fp)) {
                g_free(buffer);
                vips_error("vips__file_read", "error reading from file \"%s\"",
                    filename ? filename : "stream");
                return NULL;
            }
            break;
        }
    }

    buffer[length] = '\0';
    if (length_out)
        *length_out = length;

    return buffer;
}

typedef struct _SafeFieldIO {
    glong offset;
    int size;
    void (*copy)(gboolean swap, unsigned char *to, unsigned char *from);
} SafeFieldIO;

static void
safe_vips_copy_4byte(gboolean swap, unsigned char *to, unsigned char *from)
{
    guint32 *in = (guint32 *) from;
    guint32 *out = (guint32 *) to;

    if (swap)
        *out = GUINT32_SWAP_LE_BE(*in);
    else
        *out = *in;
}

static void
safe_vips_copy_2byte(gboolean swap, unsigned char *to, unsigned char *from)
{
    guint16 *in = (guint16 *) from;
    guint16 *out = (guint16 *) to;

    if (swap)
        *out = GUINT16_SWAP_LE_BE(*in);
    else
        *out = *in;
}

static SafeFieldIO safe_vips_header_fields[] = {
    { G_STRUCT_OFFSET(VipsImage, Xsize), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Ysize), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Bands), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Bbits), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, BandFmt), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Coding), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Type), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Xres_float), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Yres_float), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Length), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Compression), 2, safe_vips_copy_2byte },
    { G_STRUCT_OFFSET(VipsImage, Level), 2, safe_vips_copy_2byte },
    { G_STRUCT_OFFSET(VipsImage, Xoffset), 4, safe_vips_copy_4byte },
    { G_STRUCT_OFFSET(VipsImage, Yoffset), 4, safe_vips_copy_4byte }
};

VIPS_PUBLIC int
vips__read_header_bytes(VipsImage *im, unsigned char *from)
{
    gboolean swap;
    GEnumValue *value;
    int i;

    if (!im || !from)
        return -1;

    safe_vips_copy_4byte(!vips_amiMSBfirst(),
        (unsigned char *) &im->magic, from);
    from += 4;
    if (im->magic != VIPS_MAGIC_INTEL &&
        im->magic != VIPS_MAGIC_SPARC) {
        vips_error("VipsImage",
            "\"%s\" is not a VIPS image", im->filename);
        return -1;
    }

    swap = vips_amiMSBfirst() != vips_image_isMSBfirst(im);
    for (i = 0; i < (int) G_N_ELEMENTS(safe_vips_header_fields); i++) {
        safe_vips_header_fields[i].copy(swap,
            &G_STRUCT_MEMBER(unsigned char, im, safe_vips_header_fields[i].offset),
            from);
        from += safe_vips_header_fields[i].size;
    }

    im->Bbits = vips_format_sizeof(im->BandFmt) << 3;
    im->Xres = MAX(0, im->Xres_float);
    im->Yres = MAX(0, im->Yres_float);

    im->Xsize = VIPS_CLIP(1, im->Xsize, VIPS_MAX_COORD);
    im->Ysize = VIPS_CLIP(1, im->Ysize, VIPS_MAX_COORD);
    im->Bands = VIPS_CLIP(1, im->Bands, VIPS_MAX_COORD);
    im->BandFmt = VIPS_CLIP(0, im->BandFmt, VIPS_FORMAT_LAST - 1);

    value = g_enum_get_value(g_type_class_ref(VIPS_TYPE_INTERPRETATION),
        im->Type);
    if (!value || strcmp(value->value_nick, "last") == 0)
        im->Type = VIPS_INTERPRETATION_ERROR;

    value = g_enum_get_value(g_type_class_ref(VIPS_TYPE_CODING),
        im->Coding);
    if (!value || strcmp(value->value_nick, "last") == 0)
        im->Coding = VIPS_CODING_ERROR;

    switch (im->Coding) {
    case VIPS_CODING_ERROR:
        vips_error("VipsImage", "%s", "unknown coding");
        return -1;

    case VIPS_CODING_NONE:
        break;

    case VIPS_CODING_LABQ:
    case VIPS_CODING_RAD:
        if (im->Bands != 4 || im->BandFmt != VIPS_FORMAT_UCHAR) {
            vips_error("VipsImage",
                "%s", "malformed coded VIPS image");
            return -1;
        }
        break;

    default:
        vips_error("VipsImage", "%s", "unsupported coding");
        return -1;
    }

    return 0;
}

VIPS_PUBLIC int
vips__write_header_bytes(VipsImage *im, unsigned char *to)
{
    gboolean swap;
    unsigned char *q;
    int i;

    if (!im || !to)
        return -1;

    swap = vips_amiMSBfirst() != vips_image_isMSBfirst(im);
    im->Xres_float = im->Xres;
    im->Yres_float = im->Yres;

    safe_vips_copy_4byte(!vips_amiMSBfirst(),
        to, (unsigned char *) &im->magic);
    q = to + 4;

    for (i = 0; i < (int) G_N_ELEMENTS(safe_vips_header_fields); i++) {
        safe_vips_header_fields[i].copy(swap,
            q,
            &G_STRUCT_MEMBER(unsigned char, im, safe_vips_header_fields[i].offset));
        q += safe_vips_header_fields[i].size;
    }

    while (q - to < VIPS_SIZEOF_HEADER)
        *q++ = 0;

    return 0;
}

VIPS_PUBLIC int
vips__write_extension_block(VipsImage *im, void *buf, int size)
{
    gint64 length;
    gint64 psize;

    if (!im || im->fd < 0 || size < 0)
        return -1;

    psize = safe_vips_image_pixel_length(im);
    if ((length = vips_file_length(im->fd)) == -1)
        return -1;
    if (length < psize) {
        vips_error("VipsImage", "%s", "file has been truncated");
        return -1;
    }
    if (safe_vips_ftruncate(im->fd, psize) ||
        vips__seek(im->fd, psize, SEEK_SET) == -1)
        return -1;
    if (size > 0 && vips__write(im->fd, buf, (size_t) size))
        return -1;

    im->file_length = psize + size;
    return 0;
}

VIPS_PUBLIC gboolean
vips_buf_vappendf(VipsBuf *buf, const char *fmt, va_list ap)
{
    char *line;
    gboolean ok;

    line = g_strdup_vprintf(fmt, ap);
    ok = vips_buf_appends(buf, line);
    g_free(line);

    return ok;
}

VIPS_PUBLIC gboolean
vips_buf_appendf(VipsBuf *buf, const char *fmt, ...)
{
    va_list ap;
    gboolean ok;

    va_start(ap, fmt);
    ok = vips_buf_vappendf(buf, fmt, ap);
    va_end(ap);

    return ok;
}

VIPS_PUBLIC gboolean
vips_dbuf_writef(VipsDbuf *dbuf, const char *fmt, ...)
{
    va_list ap;
    char *line;
    gboolean ok;

    va_start(ap, fmt);
    line = g_strdup_vprintf(fmt, ap);
    va_end(ap);

    ok = vips_dbuf_write(dbuf, (const unsigned char *) line, strlen(line));
    g_free(line);

    return ok;
}

VIPS_PUBLIC int
vips_image_pipelinev(VipsImage *image, VipsDemandStyle hint, ...)
{
    va_list ap;
    VipsImage *value;
    VipsImage **inputs;
    int n = 0;
    int i;
    int result;

    va_start(ap, hint);
    while ((value = va_arg(ap, VipsImage *)))
        n += 1;
    va_end(ap);

    inputs = g_new0(VipsImage *, n + 1);
    va_start(ap, hint);
    for (i = 0; i < n; i++)
        inputs[i] = va_arg(ap, VipsImage *);
    va_end(ap);

    result = vips_image_pipeline_array(image, hint, inputs);
    g_free(inputs);

    return result;
}

VIPS_PUBLIC int
vips_object_set_valist(VipsObject *object, va_list ap)
{
    char *name;

    for (name = va_arg(ap, char *); name; name = va_arg(ap, char *)) {
        GParamSpec *pspec;
        VipsArgumentClass *argument_class;
        VipsArgumentInstance *argument_instance;

        if (vips_object_get_argument(VIPS_OBJECT(object), name,
                &pspec, &argument_class, &argument_instance))
            return -1;

        VIPS_ARGUMENT_COLLECT_SET(pspec, argument_class, ap);
        g_object_set_property(G_OBJECT(object),
            g_param_spec_get_name(pspec), &value);
        if (safe_vips_object_mark_argument_assigned(
                object,
                g_param_spec_get_name(pspec),
                TRUE))
            return -1;
        VIPS_ARGUMENT_COLLECT_GET(pspec, argument_class, ap);
        VIPS_ARGUMENT_COLLECT_END
    }

    return 0;
}

VIPS_PUBLIC int
vips_object_set(VipsObject *object, ...)
{
    va_list ap;
    int result;

    va_start(ap, object);
    result = vips_object_set_valist(object, ap);
    va_end(ap);

    return result;
}

VIPS_PUBLIC int
vips_object_set_from_string(VipsObject *object, const char *string)
{
    return safe_vips_object_set_from_string_internal(object, string);
}

VIPS_PUBLIC int
vips_call_required_optional(VipsOperation **operation,
    va_list required, va_list optional)
{
    int result;
    va_list a;
    va_list b;

    va_copy(a, required);
    va_copy(b, optional);
    result = safe_vips_operation_set_valist_required(*operation, a) ||
        vips_object_set_valist(VIPS_OBJECT(*operation), b);
    va_end(a);
    va_end(b);

    if (result)
        return -1;

    if (vips_cache_operation_buildp(operation))
        return -1;

    va_copy(a, required);
    va_copy(b, optional);
    result = safe_vips_operation_get_valist_required(*operation, a) ||
        safe_vips_operation_get_valist_optional(*operation, b);
    va_end(a);
    va_end(b);

    return result;
}

VIPS_PUBLIC int
vips_call(const char *operation_name, ...)
{
    VipsOperation *operation;
    int result;
    va_list required;
    va_list optional;

    if (!(operation = vips_operation_new(operation_name)))
        return -1;

    va_start(required, operation_name);
    va_copy(optional, required);

    VIPS_ARGUMENT_FOR_ALL(operation,
        pspec, argument_class, argument_instance)
    {
        g_assert(argument_instance);

        if ((argument_class->flags & VIPS_ARGUMENT_REQUIRED)) {
            VIPS_ARGUMENT_COLLECT_SET(pspec, argument_class, optional);
            VIPS_ARGUMENT_COLLECT_GET(pspec, argument_class, optional);
            VIPS_ARGUMENT_COLLECT_END
        }
    }
    VIPS_ARGUMENT_FOR_ALL_END

    g_object_unref(operation);

    result = safe_vips_call_by_name(operation_name, NULL, required, optional);

    va_end(required);
    va_end(optional);

    return result;
}

VIPS_PUBLIC int
vips_call_split(const char *operation_name, va_list optional, ...)
{
    int result;
    va_list required;

    va_start(required, optional);
    result = safe_vips_call_by_name(operation_name, NULL, required, optional);
    va_end(required);

    return result;
}

VIPS_PUBLIC int
vips_call_split_option_string(const char *operation_name,
    const char *option_string, va_list optional, ...)
{
    int result;
    va_list required;

    va_start(required, optional);
    result = safe_vips_call_by_name(operation_name, option_string, required, optional);
    va_end(required);

    return result;
}

VIPS_PUBLIC int
vips_composite(VipsImage **in, VipsImage **out, int n, int *mode, ...)
{
    VipsArrayImage *image_array;
    VipsArrayInt *mode_array;
    va_list ap;
    int result;

    if (out)
        *out = NULL;
    if (!in || !out || n < 1 || (n > 1 && !mode)) {
        vips_error("composite", "%s", "invalid composite arguments");
        return -1;
    }

    image_array = vips_array_image_new(in, n);
    mode_array = vips_array_int_new(mode, n - 1);
    if (!image_array || !mode_array) {
        if (image_array)
            vips_area_unref(VIPS_AREA(image_array));
        if (mode_array)
            vips_area_unref(VIPS_AREA(mode_array));
        return -1;
    }

    va_start(ap, mode);
    result = vips_call_split("composite", ap, image_array, out, mode_array);
    va_end(ap);

    vips_area_unref(VIPS_AREA(image_array));
    vips_area_unref(VIPS_AREA(mode_array));

    return result;
}

VIPS_PUBLIC void
vips_call_options(GOptionGroup *group, VipsOperation *operation)
{
    (void) vips_argument_map(VIPS_OBJECT(operation),
        safe_vips_call_options_add, group, NULL);
}
