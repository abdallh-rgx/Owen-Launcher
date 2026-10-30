// =============================================================================
// libue4_shim.c — the loader shim for the Owen shim injection mode.
//
// owen-patcher's `shim_patch` renames the 268MB engine to
// `lib/arm64-v8a/libUE4real.so` and puts THIS ~20KB library in the
// `libUE4.so` slot. `System.loadLibrary("UE4")` from the game's Java glue
// (com.epicgames.ue4.GameActivity) then loads the shim, whose JNI_OnLoad:
//
//   1. dlopen("libUE4real.so")              — the real engine, same namespace
//   2. calls the engine's own JNI_OnLoad     — UE4 natives registered with the VM
//   3. dlopens the Owen companion libs       — whatever is installed:
//        libOwen.so        (owen-redirecter: curl/WebView redirect layer)
//        libOwenOGS.so      (owen-ogs: the OGS 12.41 in-match server mod)
//        libOwenDumper.so   (MobileDumper-7: SDK dump for CI)
//      each missing one is skipped with a log line — one shim binary serves
//      the dump build, the play build and the product build alike.
//
// ART resolves the game's Java native methods by searching every library in
// the classloader namespace, so the engine's `Java_com_epicgames_*` symbols
// resolve transparently even though the JVM only "knows" the shim.
//
// No ELF surgery on the 268MB engine (the DT_NEEDED strtab games that caused
// the instant-crash class), no dex edits, no manifest edits.
// =============================================================================

#include <dlfcn.h>
#include <jni.h>
#include <unistd.h>
#include <android/log.h>

#define TAG "OwenShim"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, TAG, __VA_ARGS__)

typedef jint (*OnLoadFn)(JavaVM *, void *);

/* The companion libs, in load order. A missing one is skipped. */
static const char *kMods[] = {
    "libOwen.so",       /* owen-redirecter */
    "libOwenOGS.so",    /* owen-ogs (the OGS 12.41 mod) */
    "libOwenDumper.so", /* MobileDumper-7 (CI SDK dump) */
};

__attribute__((constructor))
static void shim_early(void)
{
    LOGI("libUE4 shim loaded (pid %d) — engine will be libUE4real.so", (int)getpid());
}

jint JNI_OnLoad(JavaVM *vm, void *reserved)
{
    LOGI("JNI_OnLoad: dlopen(libUE4real.so)");
    void *real = dlopen("libUE4real.so", RTLD_NOW);
    if (!real)
    {
        const char *e = dlerror();
        LOGE("dlopen(libUE4real.so) failed: %s", e ? e : "?");
        return JNI_ERR;
    }

    OnLoadFn real_onload = (OnLoadFn)dlsym(real, "JNI_OnLoad");
    if (real_onload)
    {
        jint rc = real_onload(vm, reserved);
        LOGI("engine JNI_OnLoad -> 0x%x", (unsigned)rc);
        if (rc == JNI_ERR)
            return JNI_ERR;
    }
    else
    {
        LOGI("engine exports no JNI_OnLoad (fine — ART resolves lazily)");
    }

    for (unsigned i = 0; i < sizeof(kMods) / sizeof(kMods[0]); ++i)
    {
        void *h = dlopen(kMods[i], RTLD_NOW);
        if (!h)
        {
            LOGI("%s: not installed (skipped)", kMods[i]);
            continue;
        }
        OnLoadFn m = (OnLoadFn)dlsym(h, "JNI_OnLoad");
        if (m)
            m(vm, reserved);
        LOGI("%s: loaded%s", kMods[i], m ? " (JNI_OnLoad called)" : "");
    }

    return JNI_VERSION_1_6;
}
