#!/usr/bin/env python3
"""mk_host_sdk.py — build the HOST SYNTAX-CHECK mirror of the Windows SDK.

Why this exists
===============
The Windows Dumper-7 SDK shipped with OGS 12.41
(SDK-extracted/) compiles with MSVC only.  The Android build never touches
that tree — it compiles the MobileDumper-7 dump of the same game (an
Itanium-ABI/NDK-clang SDK where these issues are fixed by the dumper).
This script produces a byte-patched MIRROR used exclusively by the host
g++ -fsyntax-only gate (see cpp/syntax_check.sh) so the ported mod code
can be type-checked on a machine without the NDK.

Every patch below is a *mechanical* MSVC-ism fix — no behavioural change
to anything the mod can observe:

  H1  UnrealContainers.hpp: the `template<typename ArrayElementType>
      friend class TAllocatedArray;` inside `TArray<ArrayElementType>`
      shadows the outer template parameter — MSVC accepts, GCC
      hard-errors.  Rename the friend parameter.
  H2  UnrealContainers.hpp: `decltype(Realloc)(...)` functional-casts
      referencing the variable in its own declarator — GCC rejects the
      decltype self-reference in a functional cast; rewrite as an
      explicit C-style pointer cast with a uintptr_t base expression
      (`__int64(x)` is not a valid functional cast under g++ either).
  H11 UnrealContainers.hpp: `TContainerIterator::operator--` calls
      `--BitIterator` on the non-dependent `FSetBitIterator`, which has
      no operator-- — a definition-time error under two-phase lookup
      that MSVC only checks at (never-occurring) instantiation; the
      iterator is unused, remove it.
  H3  UnrealContainers.hpp: `SetElement` befriends (namespace-less)
      `TSet`, which standard-wise declares ContainerImpl::TSet, not
      UC::TSet — MSVC matches UC::TSet anyway, GCC denies access.
      Make the members public (same layout, same semantics).
  H4  Basic.hpp: MSVC's <string>/<iostream> transitively provide
      <algorithm> (std::copy_n) and <cmath> (std::sqrt); libstdc++ does
      not — add the includes.
  H5  Basic.hpp: `TDelegate` primary template has a data member of the
      never-defined `struct InvalidUseOfTDelegate` — MSVC defers the
      completeness check (the primary is never instantiated), GCC checks
      at definition — give the struct an empty definition.
  H6  Basic.hpp: `static_cast<UClass*>` from `UObject*` while UClass is
      still incomplete — MSVC laxness; use reinterpret_cast (identical
      result for pointer casts between UObject types).
  H7  Basic.hpp: FField ends with `int32 ObjFlags` + 4 bytes tail
      padding.  MSVC never reuses base-class tail padding; the Itanium
      ABI (g++ *and* the Android clang build) places FProperty::ArrayDim
      inside that padding, breaking the baked offset asserts.  Add an
      explicit pad member so the MSVC offsets hold under g++ too (this
      is what MobileDumper-7 does on Android).
  H8  All SDK headers: relax `static_assert(offsetof(...)/sizeof(...)/
      alignof(...))` layout-verification asserts — MSVC-vs-Itanium ABI
      layout differs for a few hundred non-standard-layout classes; the
      Android dump carries its own clang-verified asserts.  The relaxed
      form keeps the expressions type-checked (member names, types)
      while accepting either layout.
  H9  Four enums with `: uint8` base carry a UE `_MAX = 256` sentinel
      that MSVC accepts as a non-standard extension; widen the base to
      uint16 (the _MAX sentinel value is preserved).
  H10 CoreUObject_classes.hpp: `std::string::contains` is C++23 —
      MSVC STL provides it under /std:c++latest; rewrite with
      find(...) != npos (the ported mod code does the same).

Usage:
    python3 mk_host_sdk.py <src-sdk-root> <dst-dir>
      <src-sdk-root>  dir containing SDK.hpp + SDK/ (e.g. .../SDK-extracted/SDK)
      <dst-dir>       output dir; the mirror root becomes <dst-dir>/SDK
"""
import re
import shutil
import sys
from pathlib import Path

OFF_ASSERT = re.compile(
    r"static_assert\((offsetof|sizeof|alignof)\(([^)]*)\) == (0x[0-9A-Fa-f]+|\d+),"
)


def patch(path: Path, fixes: list) -> None:
    text = path.read_text(encoding="utf-8", errors="surrogateescape")
    for i, (pattern, repl, count) in enumerate(fixes):
        text, n = re.subn(pattern, repl, text, count=count)
        if n != count:
            raise SystemExit(f"patch {i + 1} on {path.name}: expected {count} hit(s), got {n}")
    path.write_text(text, encoding="utf-8", errors="surrogateescape")


def relax_asserts(path: Path) -> int:
    text = path.read_text(encoding="utf-8", errors="surrogateescape")

    def relax(m):
        kind, inner, want = m.group(1), m.group(2), m.group(3)
        return (
            f"static_assert({kind}({inner}) == {kind}({inner}) || {kind}({inner}) == {want},"
            " /*host-mirror: MSVC layout assert relaxed*/"
        )

    text, n = OFF_ASSERT.subn(relax, text)
    if n:
        path.write_text(text, encoding="utf-8", errors="surrogateescape")
    return n


def widen_uint8_max_enums(path: Path) -> int:
    """H9 — find `enum class X : uint8 { ... = 256 }` and widen to uint16."""
    text = path.read_text(encoding="utf-8", errors="surrogateescape")
    enum_re = re.compile(r"enum class (\w+) : uint8\b")
    spans = []
    for m in enum_re.finditer(text):
        end = text.find("};", m.start())
        if end == -1:
            continue
        body = text[m.start():end]
        if re.search(r"=\s*256\b", body):
            spans.append((m.start(), end, body))
    if not spans:
        return 0
    out = []
    pos = 0
    for start, end, body in spans:
        out.append(text[pos:start])
        out.append(body.replace(": uint8", ": uint16", 1))
        pos = end
    out.append(text[pos:])
    path.write_text("".join(out), encoding="utf-8", errors="surrogateescape")
    return len(spans)


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    src = Path(sys.argv[1]).resolve()
    dst = Path(sys.argv[2]).resolve()
    if not (src / "SDK.hpp").is_file():
        raise SystemExit(f"{src} does not contain SDK.hpp")
    if dst.exists():
        shutil.rmtree(dst)
    dst.mkdir(parents=True)
    root = dst / "SDK"
    shutil.copytree(src, root)

    # H1 + H2 + H3 -------------------------------------------------------
    uc = root / "UnrealContainers.hpp"
    text = uc.read_text(encoding="utf-8", errors="surrogateescape")
    old = "\t\ttemplate<typename ArrayElementType>\n\t\tfriend class TAllocatedArray;"
    new = "\t\ttemplate<typename ArrayElementTypeFriend>\n\t\tfriend class TAllocatedArray;"
    if text.count(old) != 1:
        raise SystemExit("H1 pattern not found")
    text = text.replace(old, new)
    text = text.replace(
        "static void* (*Realloc)(void* Ptr, SIZE_T Count, uint32 Alignment) = decltype(Realloc)(__int64(GetModuleHandleW(0)) + 0x2B409C0);",
        "static void* (*Realloc)(void* Ptr, SIZE_T Count, uint32 Alignment) = (void*(*)(void*, SIZE_T, uint32))((uintptr_t)(void*)(GetModuleHandleW(0)) + 0x2B409C0);",
    )
    text = text.replace(
        "static void* (*FreeMem)(void* a1) = decltype(FreeMem)(__int64(GetModuleHandleW(0)) + 0x2B31FA0);",
        "static void* (*FreeMem)(void* a1) = (void*(*)(void*))((uintptr_t)(void*)(GetModuleHandleW(0)) + 0x2B31FA0);",
    )
    # H11: unused backwards iterator, ill-formed at definition under GCC
    old = "\t\t\tinline TContainerIterator& operator++() { ++BitIterator; return *this; }\n\t\t\tinline TContainerIterator& operator--() { --BitIterator; return *this; }"
    new = "\t\t\tinline TContainerIterator& operator++() { ++BitIterator; return *this; } /* host-mirror H11: operator-- removed */"
    if text.count(old) != 1:
        raise SystemExit("H11 pattern not found")
    text = text.replace(old, new)
    old = ("\t\ttemplate<typename SetType>\n\t\tclass SetElement\n\t\t{\n"
           "\t\tprivate:\n\t\t\ttemplate<typename SetDataType>\n\t\t\tfriend class TSet;\n"
           "\n\t\tprivate:\n\t\t\tSetType Value;")
    new = ("\t\ttemplate<typename SetType>\n\t\tclass SetElement\n\t\t{\n"
           "\t\tprivate:\n\t\t\ttemplate<typename SetDataType>\n\t\t\tfriend class TSet;\n"
           "\n\t\tpublic: /* host-mirror H3: UC::TSet friendship is MSVC-only */\n\t\t\tSetType Value;")
    if text.count(old) != 1:
        raise SystemExit("H3 pattern not found")
    text = text.replace(old, new)
    uc.write_text(text, encoding="utf-8", errors="surrogateescape")

    # H4 + H5 + H6 + H7 --------------------------------------------------
    basic = root / "SDK" / "Basic.hpp"
    patch(basic, [
        (r"#include <string>\n#include <iostream>\n#include <Windows\.h>",
         "#include <string>\n#include <iostream>\n#include <algorithm>\n#include <cmath>\n#include <Windows.h>", 1),
        (r"struct InvalidUseOfTDelegate\s+TemplateParamIsNotAFunctionSignature;",
         "struct InvalidUseOfTDelegate { } TemplateParamIsNotAFunctionSignature;", 1),
        (r"UClass\* ClassObj = static_cast<UClass\*>\(BasicFilesImpleUtils::GetObjectByIndex\(ClassIdx\)\);",
         "UClass* ClassObj = reinterpret_cast<UClass*>(BasicFilesImpleUtils::GetObjectByIndex(ClassIdx));", 2),
        (r"(\tint32\s+ObjFlags;\s*//[^\n]*\n)(\};\nstatic_assert\(alignof\(FField\))",
         r"\1\tuint8                                   Pad_34[0x4];                                       // 0x0034(0x0004)(host-mirror H7: MSVC tail-pad explicit)\n\2", 1),
    ])

    # H10 -----------------------------------------------------------------
    cou = root / "SDK" / "CoreUObject_classes.hpp"
    patch(cou, [
        (r"if \(Object->GetFullName\(\)\.contains\(FullName\)\)",
         "if (Object->GetFullName().find(FullName) != std::string::npos)", 1),
    ])

    # H8 + H9 (bulk, every header) ----------------------------------------
    relaxed = 0
    widened = 0
    for hdr in list(root.rglob("*.hpp")):
        relaxed += relax_asserts(hdr)
        widened += widen_uint8_max_enums(hdr)
    print(f"host-sdk mirror ready at {root}")
    print(f"  relaxed layout asserts : {relaxed}")
    print(f"  widened uint8 enums    : {widened}")


if __name__ == "__main__":
    main()
