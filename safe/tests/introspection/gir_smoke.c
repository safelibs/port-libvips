#include <girepository.h>
#include <glib.h>
#include <string.h>

static void
fail(GError **error, const char *message)
{
    if (error && *error) {
        g_printerr("%s: %s\n", message, (*error)->message);
        g_clear_error(error);
    }
    else {
        g_printerr("%s\n", message);
    }
}

int
main(int argc, char **argv)
{
    const char *expected_version;
    GIBaseInfo *info;
    GIFunctionInfo *function_info;
    GIRepository *repository;
    GIArgument return_value = { 0 };
    GITypelib *typelib;
    GError *error = NULL;
    const gchar *shared_library;
    const gchar *version;

    expected_version = argc > 1 ? argv[1] : "8.15.1";

    repository = g_irepository_get_default();
    typelib = g_irepository_require(repository, "Vips", "8.0", 0, &error);
    if (!typelib) {
        fail(&error, "unable to load the Vips typelib");
        return 1;
    }

    shared_library = g_irepository_get_shared_library(repository, "Vips");
    if (!shared_library || strcmp(shared_library, "libvips.so.42") != 0) {
        g_printerr(
            "unexpected Vips shared library metadata: expected libvips.so.42, found %s\n",
            shared_library ? shared_library : "(null)");
        return 1;
    }

    info = g_irepository_find_by_name(repository, "Vips", "version_string");
    if (!info) {
        g_printerr("unable to locate Vips.version_string() in typelib\n");
        return 1;
    }

    if (g_base_info_get_type(info) != GI_INFO_TYPE_FUNCTION) {
        g_printerr("Vips.version_string is not a function in the typelib\n");
        g_base_info_unref(info);
        return 1;
    }

    function_info = (GIFunctionInfo *) info;
    if (!g_function_info_invoke(function_info,
            NULL,
            0,
            NULL,
            0,
            &return_value,
            &error)) {
        fail(&error, "unable to invoke Vips.version_string()");
        g_base_info_unref(info);
        return 1;
    }

    version = return_value.v_string;
    if (!version || strcmp(version, expected_version) != 0) {
        g_printerr(
            "unexpected libvips version string: expected %s, found %s\n",
            expected_version,
            version ? version : "(null)");
        g_base_info_unref(info);
        return 1;
    }

    g_base_info_unref(info);
    g_print("loaded Vips typelib against %s with version %s\n", shared_library, version);
    return 0;
}
