/*
 * stdlib.h — minimal bionic-compatible shim (see assert.h for the rationale).
 * ring's C core includes <stdlib.h> only on the MSVC code path, so this shim
 * exists purely so the include never fails; abort() is declared for
 * completeness.
 */
#ifndef _OWEN_NDK_SHIM_STDLIB_H
#define _OWEN_NDK_SHIM_STDLIB_H

#include <stddef.h> /* compiler-provided (clang builtin include dir) */

#ifdef __cplusplus
extern "C" {
#endif

void abort(void);

#ifdef __cplusplus
}
#endif

#endif /* _OWEN_NDK_SHIM_STDLIB_H */
