/*
 * assert.h — minimal bionic-compatible shim.
 *
 * Why this exists: the NDK at /home/z/ndk/android-ndk-r27c ships the full
 * toolchain and libc.a, but its sysroot has NO C library headers
 * (sysroot/usr/include is absent). The only C compiled for the aarch64-linux-
 * android target in this workspace is ring's crypto core, which needs
 * <assert.h>, <string.h> (and <stdlib.h> on paper). The compiler-provided
 * headers (stdint.h, stddef.h, stdalign.h, float.h, limits.h, ...) come from
 * clang's builtin include directory and are fine.
 *
 * This shim is wired in through CFLAGS_aarch64-linux-android in
 * .cargo/config.toml. It intentionally declares only what the workspace's C
 * actually uses; the declarations match bionic's ABI (aarch64 LP64).
 */
#ifndef _OWEN_NDK_SHIM_ASSERT_H
#define _OWEN_NDK_SHIM_ASSERT_H

/* bionic: __assert2(file, line, func, expr) — noreturn. */
void __assert2(const char*, int, const char*, const char*);

#undef assert

#ifdef NDEBUG
#define assert(expr) ((void)0)
#else
#define assert(expr) \
    ((expr) ? (void)0 : __assert2(__FILE__, __LINE__, __func__, #expr))
#endif

#endif /* _OWEN_NDK_SHIM_ASSERT_H */
