#include <glib.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
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
extern int safe_vips_object_mark_argument_assigned(
    VipsObject *object, const char *name, gboolean assigned);

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

            if (G_IS_PARAM_SPEC_OBJECT(pspec)) {
                GObject *object;

                object = *((GObject **) arg);
                if (object)
                    g_object_unref(object);
            }

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

            if (G_IS_PARAM_SPEC_OBJECT(pspec)) {
                GObject *object;

                object = *((GObject **) arg);
                if (object)
                    g_object_unref(object);
            }
        }

        VIPS_ARGUMENT_COLLECT_END
    }

    return 0;
}

static int
safe_vips_call_by_name(const char *operation_name,
    const char *option_string, va_list required, va_list optional)
{
    VipsOperation *operation;
    int result;

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

VIPS_PUBLIC int
vips_image_write_to_target(VipsImage *in, const char *suffix, VipsTarget *target, ...)
{
    return safe_vips_image_write_to_target_internal(in, suffix, target);
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

VIPS_PUBLIC void
vips_call_options(GOptionGroup *group, VipsOperation *operation)
{
    (void) vips_argument_map(VIPS_OBJECT(operation),
        safe_vips_call_options_add, group, NULL);
}
