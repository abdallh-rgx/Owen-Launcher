// Windows.h — HOST SYNTAX-CHECK shim (cpp/host_stubs/Windows.h)
//
// The Windows Dumper-7 SDK's Basic.hpp does `#include <Windows.h>` (angle
// include), so this file wins on the host include path and provides the
// minimal MSVC surface the SDK itself needs.  It is NEVER compiled on
// Android: the NDK build (build.rs) does not put cpp/host_stubs on the
// include path for SDK TUs, and the mod's own code never includes
// Windows.h — the port uses OgsCompat.h exclusively.
//
// Provided here because g++/libstdc++ differs from MSVC STL:
//   * <cstdint> in the global namespace (MSVC's <string> leaks it),
//   * __int8/__int16/__int32/__int64 spelled types (macros so that
//     `unsigned __int8` expands correctly — the mingw trick),
//   * GetModuleHandle{,A,W} dummies: the SDK's static initializers
//     (UC::Realloc / UC::FreeMem) and InSDKUtils::GetImageBase call them;
//     on the host they return a fake base so the pointers are inert.
//     The MOD never uses them (Ogs::Base()/Ogs::Resolve() instead).
#pragma once
#include <cstddef>
#include <cstdint>
#include <cstring>

#define __int8 char
#define __int16 short
#define __int32 int
#define __int64 long long

typedef unsigned long DWORD;
typedef unsigned char BYTE;
typedef unsigned short WORD;
typedef int BOOL;
typedef void* LPVOID;
typedef void* PVOID;
typedef void* HANDLE;
typedef char TCHAR;
typedef size_t SIZE_T;
typedef unsigned int UINT;
typedef void* HMODULE;
typedef unsigned long ULONG;
typedef unsigned long ULONG_PTR;
typedef unsigned long DWORD_PTR;
typedef void* FARPROC;
typedef wchar_t WCHAR;
typedef wchar_t* LPWSTR;
typedef const wchar_t* LPCWSTR;
typedef char* LPSTR;
typedef const char* LPCSTR;

#ifndef TRUE
#define TRUE 1
#endif
#ifndef FALSE
#define FALSE 0
#endif
#ifndef MAX_PATH
#define MAX_PATH 260
#endif
#ifndef WINAPI
#define WINAPI
#endif
#ifndef CALLBACK
#define CALLBACK
#endif
#ifndef CP_UTF8
#define CP_UTF8 65001
#endif

inline HMODULE GetModuleHandleW(const void*) { return (HMODULE)(uintptr_t)0; }
inline HMODULE GetModuleHandleA(const char*) { return (HMODULE)(uintptr_t)0; }
inline HMODULE GetModuleHandle(const char*) { return (HMODULE)(uintptr_t)0; }
inline int MultiByteToWideChar(unsigned, unsigned long, const char*, int, wchar_t*, int) { return 0; }
