#!/usr/bin/env python3
"""
tools/dex/make_dex.py — deterministic handcrafted DEX emitter.

Produces `assets/dex/classes.dex` (DEX version 035) containing exactly ONE
class:

    com.abdallhrgx.owenlauncher.core.patch.InstallResultReceiver
        extends android.content.BroadcastReceiver

with:

  * `<clinit>()V`   — `const-string v0, "owen_app"` ;
                      `invoke-static {v0}, Ljava/lang/System;->loadLibrary(Ljava/lang/String;)V` ;
                      `return-void`                     (registers 1, ins 0)
  * `<init>()V`     — `invoke-direct {p0}, Landroid/content/BroadcastReceiver;-><init>()V` ;
                      `return-void`                     (registers 1, ins 1)
  * `onReceive(Landroid/content/Context;Landroid/content/Intent;)V` public —
                      `invoke-static {p2}, …InstallResultReceiver;->nativeOnReceive(Landroid/content/Intent;)V` ;
                      `return-void`                     (registers 3, ins 3)
  * `nativeOnReceive(Landroid/content/Intent;)V` public static native —
                      declared, no code (code_off = 0)

`System.loadLibrary("owen_app")` in `<clinit>` pulls in `libowen_app.so`
(owen-app's cdylib), whose `JNI_OnLoad` RegisterNatives-es `nativeOnReceive` —
the Rust twin of the original Kotlin `InstallResultReceiver` (see
crates/owen-app/src/jni_android.rs).

The emitter is followed by a MANDATORY self-check that re-parses the emitted
file from scratch (header → ids → class_def → class_data → code items →
map_list) and fails loudly on any inconsistency.

Usage: python3 tools/dex/make_dex.py [output-path]
       (default: <repo>/assets/dex/classes.dex)
"""

import hashlib
import struct
import sys
import zlib
from pathlib import Path

# --------------------------------------------------------------------------
# The spec of the class (single source of truth for emitter AND validator)
# --------------------------------------------------------------------------

CLASS_DESC = "Lcom/abdallhrgx/owenlauncher/core/patch/InstallResultReceiver;"
SUPER_DESC = "Landroid/content/BroadcastReceiver;"
CONTEXT_DESC = "Landroid/content/Context;"
INTENT_DESC = "Landroid/content/Intent;"
STRING_DESC = "Ljava/lang/String;"
SYSTEM_DESC = "Ljava/lang/System;"
VOID_DESC = "V"

LIB_NAME = "owen_app"  # System.loadLibrary("owen_app") — matches the manifest's
                       # android.app.lib_name and the owen-app cdylib name.

ACC_PUBLIC = 0x1
ACC_STATIC = 0x8
ACC_NATIVE = 0x100
ACC_CONSTRUCTOR = 0x10000

CLASS_FLAGS = ACC_PUBLIC                          # 0x0000_0001
CLINIT_FLAGS = ACC_PUBLIC | ACC_STATIC | ACC_CONSTRUCTOR  # 0x0001_0009
INIT_FLAGS = ACC_PUBLIC | ACC_CONSTRUCTOR         # 0x0001_0001
ONRECEIVE_FLAGS = ACC_PUBLIC                      # 0x0000_0001
NATIVE_FLAGS = ACC_PUBLIC | ACC_STATIC | ACC_NATIVE     # 0x0000_0109

# Dalvik opcodes used by the three code items.
OP_RETURN_VOID = 0x0E
OP_CONST_STRING = 0x1A  # format 21c
OP_INVOKE_DIRECT = 0x70  # format 35c
OP_INVOKE_STATIC = 0x71  # format 35c

# map_list item type codes
TYPE_HEADER_ITEM = 0x0000
TYPE_STRING_ID_ITEM = 0x0001
TYPE_TYPE_ID_ITEM = 0x0002
TYPE_PROTO_ID_ITEM = 0x0003
TYPE_FIELD_ID_ITEM = 0x0004
TYPE_METHOD_ID_ITEM = 0x0005
TYPE_CLASS_DEF_ITEM = 0x0006
TYPE_MAP_LIST = 0x1000
TYPE_TYPE_LIST = 0x1001
TYPE_CODE_ITEM = 0x2001
TYPE_STRING_DATA_ITEM = 0x2002
TYPE_CLASS_DATA_ITEM = 0x2000

NO_INDEX = 0xFFFFFFFF
HEADER_SIZE = 0x70
ENDIAN_TAG = 0x12345678


# --------------------------------------------------------------------------
# Low-level encoders
# --------------------------------------------------------------------------

def uleb(n: int) -> bytes:
    """ULEB128 (used for string lengths and class_data fields)."""
    if n < 0 or n >= (1 << 64):
        raise ValueError(f"uleb128 out of range: {n}")
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def read_uleb(data: bytes, off: int):
    """Decode a ULEB128 at `off`; returns (value, next_off)."""
    result = 0
    shift = 0
    while True:
        b = data[off]
        off += 1
        result |= (b & 0x7F) << shift
        if not (b & 0x80):
            return result, off
        shift += 7
        if shift > 63:
            raise ValueError("uleb128 too long")


def mutf8(s: str):
    """Encode `s` as MUTF-8; returns (utf16_unit_count, bytes)."""
    units = []
    for ch in s:
        cp = ord(ch)
        if cp < 0x10000:
            units.append(cp)
        else:  # surrogate pair (not needed for our ASCII strings, but correct)
            cp -= 0x10000
            units.append(0xD800 + (cp >> 10))
            units.append(0xDC00 + (cp & 0x3FF))
    out = bytearray()
    for u in units:
        if u == 0:
            out += b"\xc0\x80"  # MUTF-8 encodes U+0000 as C0 80
        elif u < 0x80:
            out.append(u)
        elif u < 0x800:
            out.append(0xC0 | (u >> 6))
            out.append(0x80 | (u & 0x3F))
        else:
            out.append(0xE0 | (u >> 12))
            out.append(0x80 | ((u >> 6) & 0x3F))
            out.append(0x80 | (u & 0x3F))
    return len(units), bytes(out)


def utf16_key(s: str) -> bytes:
    """Sort key: the UTF-16 code-unit sequence (DEX string sort order)."""
    return s.encode("utf-16-be")


# --------------------------------------------------------------------------
# Instruction builders (code units, little-endian u16 each when serialized)
# --------------------------------------------------------------------------

def insn_const_string(reg: int, string_idx: int):
    """format 21c: AA|op BBBB."""
    assert 0 <= reg <= 0xFF and 0 <= string_idx <= 0xFFFF
    return [OP_CONST_STRING | (reg << 8), string_idx]


def insn_invoke(kind_op: int, regs, method_idx: int):
    """format 35c: A|G|op BBBB F|E|D|C — up to 5 argument registers."""
    assert 1 <= len(regs) <= 5, "35c takes 1..5 registers (0-arg invoke does not exist)"
    assert 0 <= method_idx <= 0xFFFF
    for r in regs:
        assert 0 <= r <= 0xF, "35c argument registers must fit a nibble"
    a = len(regs)
    unit0 = kind_op | (a << 12)  # G nibble (5th register) unused: A<=1 here
    unit2 = 0
    for i, r in enumerate(regs[:4]):
        unit2 |= r << (4 * i)
    return [unit0, method_idx, unit2]


def code_item(registers: int, ins: int, outs: int, insns) -> bytes:
    """serialized code_item: header + insns (+ 2-byte pad if odd)."""
    body = struct.pack(
        "<HHHHII",
        registers,
        ins,
        outs,
        0,  # tries_size
        0,  # debug_info_off
        len(insns),
    )
    body += b"".join(struct.pack("<H", u) for u in insns)
    if len(insns) % 2:
        body += b"\x00\x00"  # keep insns 4-byte padded
    return body


# --------------------------------------------------------------------------
# Emitter
# --------------------------------------------------------------------------

def build_dex() -> bytes:
    # ---- 1. the string pool (sorted by UTF-16 code-unit order) ------------
    strings = [
        "()V",
        "(Landroid/content/Context;Landroid/content/Intent;)V",
        "(Landroid/content/Intent;)V",
        "(Ljava/lang/String;)V",
        "<clinit>",
        "<init>",
        SUPER_DESC,
        CONTEXT_DESC,
        INTENT_DESC,
        CLASS_DESC,
        STRING_DESC,
        SYSTEM_DESC,
        "V",
        "VL",
        "VLL",
        "loadLibrary",
        "nativeOnReceive",
        "onReceive",
        LIB_NAME,
    ]
    strings = sorted(strings, key=utf16_key)
    S = {s: i for i, s in enumerate(strings)}

    # ---- 2. type ids (sorted by string index) ------------------------------
    type_descs = sorted(
        [SUPER_DESC, CONTEXT_DESC, INTENT_DESC, CLASS_DESC, STRING_DESC, SYSTEM_DESC, VOID_DESC],
        key=lambda d: S[d],
    )
    T = {d: i for i, d in enumerate(type_descs)}

    # ---- 3. proto ids -------------------------------------------------------
    # (return type, parameter type descriptors) — kept in the dx-compatible
    # sort order (return_type_idx, then parameters element-wise).
    protos = [
        (VOID_DESC, []),
        (VOID_DESC, [CONTEXT_DESC, INTENT_DESC]),
        (VOID_DESC, [INTENT_DESC]),
        (VOID_DESC, [STRING_DESC]),
    ]

    def shorty_of(ret: str, params) -> str:
        def ch(d: str) -> str:
            return "L" if d.startswith(("L", "[")) else d
        return ch(ret) + "".join(ch(p) for p in params)

    # ---- 4. method ids ------------------------------------------------------
    # (class descriptor, name, proto index) — sorted the dx-compatible way:
    # (class_idx, name_idx, proto_idx).
    method_defs = [
        (SUPER_DESC, "<init>", 0),
        (CLASS_DESC, "<clinit>", 0),
        (CLASS_DESC, "<init>", 0),
        (CLASS_DESC, "nativeOnReceive", 2),
        (CLASS_DESC, "onReceive", 1),
        (SYSTEM_DESC, "loadLibrary", 3),
    ]
    method_defs.sort(key=lambda m: (T[m[0]], S[m[1]], m[2]))
    M = {m[:2]: i for i, m in enumerate(method_defs)}

    # ---- 5. static layout arithmetic ---------------------------------------
    n_strings = len(strings)
    n_types = len(type_descs)
    n_protos = len(protos)
    n_methods = len(method_defs)

    string_ids_off = HEADER_SIZE
    type_ids_off = string_ids_off + 4 * n_strings
    proto_ids_off = type_ids_off + 4 * n_types
    method_ids_off = proto_ids_off + 12 * n_protos
    field_ids_size = 0
    field_ids_off = 0
    class_defs_off = method_ids_off + 8 * n_methods
    code_off_first = class_defs_off + 32  # exactly one class_def

    # code items (fixed sizes: 16-byte header + 2 bytes/unit, padded to 4)
    clinit_insns = (
        insn_const_string(0, S[LIB_NAME])
        + insn_invoke(OP_INVOKE_STATIC, [0], M[(SYSTEM_DESC, "loadLibrary")])
        + [OP_RETURN_VOID]
    )
    init_insns = (
        insn_invoke(OP_INVOKE_DIRECT, [0], M[(SUPER_DESC, "<init>")])
        + [OP_RETURN_VOID]
    )
    onreceive_insns = (
        insn_invoke(OP_INVOKE_STATIC, [2], M[(CLASS_DESC, "nativeOnReceive")])
        + [OP_RETURN_VOID]
    )

    clinit_code = code_item(registers=1, ins=0, outs=1, insns=clinit_insns)
    init_code = code_item(registers=1, ins=1, outs=1, insns=init_insns)
    onreceive_code = code_item(registers=3, ins=3, outs=1, insns=onreceive_insns)

    clinit_code_off = code_off_first
    init_code_off = clinit_code_off + len(clinit_code)
    onreceive_code_off = init_code_off + len(init_code)
    code_section_end = onreceive_code_off + len(onreceive_code)

    # ---- 6. string data items (emitted in string_ids order) -----------------
    string_data = bytearray()
    string_data_offs = []
    for s in strings:
        string_data_offs.append(code_section_end + len(string_data))
        n_units, raw = mutf8(s)
        string_data += uleb(n_units) + raw + b"\x00"
    string_data_start = string_data_offs[0]

    # ---- 7. type lists (one per non-empty proto param list, 4-aligned) ------
    def align4(n: int) -> int:
        return (n + 3) & ~3

    type_lists = bytearray()
    type_list_offs = [0] * n_protos
    cursor = align4(code_section_end + len(string_data))
    type_lists_start = cursor
    for i, (ret, params) in enumerate(protos):
        if not params:
            type_list_offs[i] = 0  # no params: params_off = 0
            continue
        item = struct.pack("<I", len(params))
        item += b"".join(struct.pack("<H", T[p]) for p in params)
        type_list_offs[i] = cursor + len(type_lists)
        type_lists += item
        # pad each list to a 4-byte multiple so the next one stays aligned
        type_lists += b"\x00" * (align4(len(item)) - len(item))
    type_lists_end = type_lists_start + len(type_lists)

    # ---- 8. class_data ------------------------------------------------------
    # direct : <clinit>, <init>, nativeOnReceive (ascending method_idx)
    # virtual: onReceive
    direct = [
        (M[(CLASS_DESC, "<clinit>")], CLINIT_FLAGS, clinit_code_off),
        (M[(CLASS_DESC, "<init>")], INIT_FLAGS, init_code_off),
        (M[(CLASS_DESC, "nativeOnReceive")], NATIVE_FLAGS, 0),
    ]
    virtual = [
        (M[(CLASS_DESC, "onReceive")], ONRECEIVE_FLAGS, onreceive_code_off),
    ]
    assert [m[0] for m in direct] == sorted(m[0] for m in direct)
    assert [m[0] for m in virtual] == sorted(m[0] for m in virtual)

    class_data = bytearray()
    class_data += uleb(0) + uleb(0) + uleb(len(direct)) + uleb(len(virtual))
    prev = 0
    for idx, flags, code_off in direct:
        class_data += uleb(idx - prev) + uleb(flags) + uleb(code_off)
        prev = idx
    prev = 0
    for idx, flags, code_off in virtual:
        class_data += uleb(idx - prev) + uleb(flags) + uleb(code_off)
        prev = idx
    class_data_off = type_lists_end  # already 4-aligned

    # ---- 9. map list --------------------------------------------------------
    n_type_lists = sum(1 for _, params in protos if params)
    map_entries = [
        (TYPE_HEADER_ITEM, 1, 0),
        (TYPE_STRING_ID_ITEM, n_strings, string_ids_off),
        (TYPE_TYPE_ID_ITEM, n_types, type_ids_off),
        (TYPE_PROTO_ID_ITEM, n_protos, proto_ids_off),
        (TYPE_METHOD_ID_ITEM, n_methods, method_ids_off),
        (TYPE_CLASS_DEF_ITEM, 1, class_defs_off),
        (TYPE_CODE_ITEM, 3, clinit_code_off),
        (TYPE_STRING_DATA_ITEM, n_strings, string_data_start),
        (TYPE_TYPE_LIST, n_type_lists, type_lists_start),
        (TYPE_CLASS_DATA_ITEM, 1, class_data_off),
    ]
    # MAP_LIST itself is last; its offset needs the map size, which is fixed:
    map_size = 4 + 12 * (len(map_entries) + 1)
    map_off = align4(class_data_off + len(class_data))
    map_entries.append((TYPE_MAP_LIST, 1, map_off))
    assert [e[2] for e in map_entries] == sorted(e[2] for e in map_entries)

    file_size = map_off + map_size
    data_off = clinit_code_off  # the data section starts right after class_defs
    data_size = file_size - data_off

    # ---- 10. serialize ------------------------------------------------------
    out = bytearray(b"\x00" * file_size)

    def put(off: int, blob: bytes):
        out[off : off + len(blob)] = blob

    # header (checksum + signature patched at the very end)
    put(
        0,
        struct.pack(
            "<8sI20sI"  # magic, checksum, signature, file_size
            "IIII"      # header_size, endian_tag, link_size, link_off
            "I"         # map_off
            "II"        # string_ids_size, string_ids_off
            "II"        # type_ids_size, type_ids_off
            "II"        # proto_ids_size, proto_ids_off
            "II"        # field_ids_size, field_ids_off
            "II"        # method_ids_size, method_ids_off
            "II"        # class_defs_size, class_defs_off
            "II",       # data_size, data_off
            b"dex\n035\x00",
            0,
            b"\x00" * 20,
            file_size,
            HEADER_SIZE,
            ENDIAN_TAG,
            0,
            0,
            map_off,
            n_strings,
            string_ids_off,
            n_types,
            type_ids_off,
            n_protos,
            proto_ids_off,
            field_ids_size,
            field_ids_off,
            n_methods,
            method_ids_off,
            1,
            class_defs_off,
            data_size,
            data_off,
        ),
    )

    # string_ids
    for i, off in enumerate(string_data_offs):
        put(string_ids_off + 4 * i, struct.pack("<I", off))

    # type_ids
    for i, d in enumerate(type_descs):
        put(type_ids_off + 4 * i, struct.pack("<I", S[d]))

    # proto_ids
    for i, (ret, params) in enumerate(protos):
        put(
            proto_ids_off + 12 * i,
            struct.pack("<III", S[shorty_of(ret, params)], T[ret], type_list_offs[i]),
        )

    # method_ids
    for i, (cls, name, proto_idx) in enumerate(method_defs):
        put(
            method_ids_off + 8 * i,
            struct.pack("<HHI", T[cls], proto_idx, S[name]),
        )

    # class_def
    put(
        class_defs_off,
        struct.pack(
            "<IIIIIIII",
            T[CLASS_DESC],      # class_idx
            CLASS_FLAGS,        # access_flags
            T[SUPER_DESC],      # superclass_idx
            0,                  # interfaces_off (none)
            NO_INDEX,           # source_file_idx
            0,                  # annotations_off (none)
            class_data_off,     # class_data_off
            0,                  # static_values_off (no static fields)
        ),
    )

    # code items
    put(clinit_code_off, clinit_code)
    put(init_code_off, init_code)
    put(onreceive_code_off, onreceive_code)

    # string data
    put(string_data_start, bytes(string_data))

    # type lists
    put(type_lists_start, bytes(type_lists))

    # class data
    put(class_data_off, bytes(class_data))

    # map list
    map_blob = struct.pack("<I", len(map_entries))
    for item_type, size, off in map_entries:
        map_blob += struct.pack("<HHII", item_type, 0, size, off)
    put(map_off, map_blob)

    # ---- 11. checksum + signature -------------------------------------------
    signature = hashlib.sha1(bytes(out[32:])).digest()
    put(12, signature)
    checksum = zlib.adler32(bytes(out[12:])) & 0xFFFFFFFF
    put(8, struct.pack("<I", checksum))

    return bytes(out)


# --------------------------------------------------------------------------
# Validator — re-parses the emitted bytes from scratch and checks everything
# --------------------------------------------------------------------------

class Fail(Exception):
    pass


def validate(data: bytes) -> list:
    report = []

    def check(cond, msg):
        report.append(("PASS" if cond else "FAIL", msg))
        if not cond:
            raise Fail(msg)

    # ---- header ----
    check(data[:8] == b"dex\n035\x00", "magic: dex\\n035\\0")
    (file_size,) = struct.unpack_from("<I", data, 0x20)
    (header_size,) = struct.unpack_from("<I", data, 0x24)
    (endian,) = struct.unpack_from("<I", data, 0x28)
    check(file_size == len(data), f"file_size {file_size} == actual {len(data)}")
    check(header_size == 0x70, "header_size == 0x70")
    check(endian == 0x12345678, "endian_tag == 0x12345678")
    (stored_sum,) = struct.unpack_from("<I", data, 8)
    calc_sum = zlib.adler32(data[12:]) & 0xFFFFFFFF
    check(stored_sum == calc_sum, f"checksum adler32(bytes[12:]) == {stored_sum:#010x}")
    check(
        data[12:32] == hashlib.sha1(data[32:]).digest(),
        "signature sha1(bytes[32:]) matches",
    )
    (
        link_size, link_off, map_off,
        n_strings, string_ids_off,
        n_types, type_ids_off,
        n_protos, proto_ids_off,
        n_fields, field_ids_off,
        n_methods, method_ids_off,
        n_class_defs, class_defs_off,
        data_size, data_off,
    ) = struct.unpack_from("<17I", data, 0x2C)
    check(link_size == 0 and link_off == 0, "link_size/link_off == 0")
    check(n_fields == 0 and field_ids_off == 0, "no field_ids")
    check(n_class_defs == 1, "exactly one class_def")
    check(n_strings == 19 and n_types == 7 and n_protos == 4 and n_methods == 6,
          f"id counts (strings={n_strings}, types={n_types}, protos={n_protos}, methods={n_methods})")

    # ---- strings ----
    def read_string(off: int):
        n_units, off2 = read_uleb(data, off)
        end = data.index(b"\x00", off2)
        return data[off2:end].decode("utf-8"), n_units, end + 1

    strings = []
    for i in range(n_strings):
        (s_off,) = struct.unpack_from("<I", data, string_ids_off + 4 * i)
        check(0 < s_off < len(data), f"string_id[{i}] offset {s_off} in range")
        s, n_units, _ = read_string(s_off)
        check(n_units == len(s), f"string[{i}] {s!r}: uleb utf16 length correct")
        strings.append(s)
    check(
        [utf16_key(s) for s in strings] == sorted(utf16_key(s) for s in strings)
        and len(set(strings)) == len(strings),
        "string_ids sorted by UTF-16 code-unit order (strictly)",
    )
    S = {s: i for i, s in enumerate(strings)}
    for needed in ("()V", "<clinit>", "<init>", "loadLibrary", "nativeOnReceive",
                   "onReceive", "owen_app", "V", "VL", "VLL",
                   CLASS_DESC, SUPER_DESC, CONTEXT_DESC, INTENT_DESC,
                   STRING_DESC, SYSTEM_DESC):
        check(needed in S, f"string pool contains {needed!r}")

    # ---- types ----
    type_idxs = [struct.unpack_from("<I", data, type_ids_off + 4 * i)[0] for i in range(n_types)]
    check(type_idxs == sorted(type_idxs), "type_ids sorted by string index")
    types = [strings[i] for i in type_idxs]
    check(SUPER_DESC in types and CLASS_DESC in types and VOID_DESC in types,
          "type_ids contain super / class / V")

    def t_idx(desc: str) -> int:
        return type_idxs[types.index(desc)]

    # ---- protos ----
    def read_type_list(off: int):
        (n,) = struct.unpack_from("<I", data, off)
        entries = [struct.unpack_from("<H", data, off + 4 + 2 * i)[0] for i in range(n)]
        return [types[e] for e in entries]

    protos = []
    for i in range(n_protos):
        shorty_idx, ret_idx, params_off = struct.unpack_from("<III", data, proto_ids_off + 12 * i)
        ret = types[ret_idx]
        params = read_type_list(params_off) if params_off else []
        if params_off:
            check(params_off % 4 == 0 and params_off < len(data),
                  f"proto[{i}] params_off {params_off:#x} 4-aligned and in range")
        shorty = strings[shorty_idx]
        expected = ("L" if ret.startswith(("L", "[")) else ret) + "".join(
            "L" if p.startswith(("L", "[")) else p for p in params)
        check(shorty == expected, f"proto[{i}] shorty {shorty!r} matches ({ret}, {params})")
        protos.append((ret, params))
    check((VOID_DESC, []) in protos, "proto ()V present")
    check((VOID_DESC, [CONTEXT_DESC, INTENT_DESC]) in protos, "proto (Context,Intent)V present")
    check((VOID_DESC, [INTENT_DESC]) in protos, "proto (Intent)V present")
    check((VOID_DESC, [STRING_DESC]) in protos, "proto (String)V present")

    # ---- methods ----
    methods = []
    for i in range(n_methods):
        class_idx, proto_idx, name_idx = struct.unpack_from("<HHI", data, method_ids_off + 8 * i)
        check(class_idx < n_types and proto_idx < n_protos and name_idx < n_strings,
              f"method_id[{i}] indices in range")
        methods.append((types[class_idx], strings[name_idx], protos[proto_idx]))
    for expected in (
        (SYSTEM_DESC, "loadLibrary", (VOID_DESC, [STRING_DESC])),
        (SUPER_DESC, "<init>", (VOID_DESC, [])),
        (CLASS_DESC, "<clinit>", (VOID_DESC, [])),
        (CLASS_DESC, "<init>", (VOID_DESC, [])),
        (CLASS_DESC, "nativeOnReceive", (VOID_DESC, [INTENT_DESC])),
        (CLASS_DESC, "onReceive", (VOID_DESC, [CONTEXT_DESC, INTENT_DESC])),
    ):
        check(expected in methods, f"method_id present: {expected[1]}")

    def m_idx(cls, name):
        return methods.index(next(m for m in methods if m[0] == cls and m[1] == name))

    # ---- class def ----
    (c_class, c_flags, c_super, c_ifaces, c_source, c_annot, c_data_off,
     c_static_values) = struct.unpack_from("<8I", data, class_defs_off)
    check(types[c_class] == CLASS_DESC, "class_def: the receiver class")
    check(c_flags == 0x1, "class_def access_flags == 0x1 (PUBLIC)")
    check(types[c_super] == SUPER_DESC, "class_def superclass == BroadcastReceiver")
    check(c_ifaces == 0 and c_source == NO_INDEX and c_annot == 0 and c_static_values == 0,
          "class_def: no interfaces / source / annotations / static values")
    check(0 < c_data_off < len(data), "class_def class_data_off in range")

    # ---- class data ----
    off = c_data_off
    n_static_fields, off = read_uleb(data, off)
    n_instance_fields, off = read_uleb(data, off)
    n_direct, off = read_uleb(data, off)
    n_virtual, off = read_uleb(data, off)
    check((n_static_fields, n_instance_fields, n_direct, n_virtual) == (0, 0, 3, 1),
          f"class_data sizes: sf={n_static_fields} if={n_instance_fields} "
          f"dm={n_direct} vm={n_virtual} (3 direct: <clinit>/<init>/nativeOnReceive)")

    def read_encoded_methods(off, count):
        entries = []
        prev = 0
        for _ in range(count):
            diff, off = read_uleb(data, off)
            flags, off = read_uleb(data, off)
            code_off, off = read_uleb(data, off)
            idx = prev + diff
            entries.append((idx, flags, code_off))
            prev = idx
        return entries, off

    direct, off = read_encoded_methods(off, n_direct)
    virtual, off = read_encoded_methods(off, n_virtual)
    check(off <= len(data), "class_data stream consumed within the file")
    idxs = [e[0] for e in direct]
    check(idxs == sorted(idxs), "direct methods sorted by method_idx")
    check([e[0] for e in virtual] == sorted(e[0] for e in virtual), "virtual methods sorted")

    by_idx = {}
    for idx, flags, code_off in direct + virtual:
        cls, name, proto = methods[idx]
        by_idx[name] = (flags, code_off)

    check(by_idx["<clinit>"][0] == 0x10009, "<clinit> flags == 0x10009 (PUBLIC|STATIC|CONSTRUCTOR)")
    check(by_idx["<init>"][0] == 0x10001, "<init> flags == 0x10001 (PUBLIC|CONSTRUCTOR)")
    check(by_idx["nativeOnReceive"][0] == 0x109, "nativeOnReceive flags == 0x109 (PUBLIC|STATIC|NATIVE)")
    check(by_idx["nativeOnReceive"][1] == 0, "nativeOnReceive code_off == 0 (native, no code)")
    check(by_idx["onReceive"][0] == 0x1, "onReceive flags == 0x1 (PUBLIC)")

    # ---- code items ----
    def read_code_item(off):
        regs, ins, outs, tries, debug_off, insns_size = struct.unpack_from("<HHHHII", data, off)
        insns = [struct.unpack_from("<H", data, off + 16 + 2 * i)[0] for i in range(insns_size)]
        return regs, ins, outs, tries, debug_off, insns_size, insns

    def walk_insns(insns):
        """Generic structural walk: returns a (opcode, operands) list."""
        units = []
        i = 0
        while i < len(insns):
            op = insns[i] & 0xFF
            if op == 0x0E:  # return-void (10x)
                units.append(("return-void",)); i += 1
            elif op == 0x1A:  # const-string (21c)
                units.append(("const-string", (insns[i] >> 8) & 0xFF, insns[i + 1])); i += 2
            elif op in (0x6E, 0x6F, 0x70, 0x71, 0x72):  # invoke-* (35c)
                a = (insns[i] >> 12) & 0xF
                regs = [(insns[i + 2] >> (4 * k)) & 0xF for k in range(min(a, 4))]
                units.append((f"invoke@{op:#04x}", a, insns[i + 1], regs)); i += 3
            else:
                raise Fail(f"unexpected opcode {op:#04x} in insns walk")
        return units

    # <clinit>
    regs, ins, outs, tries, debug_off, insns_size, insns = read_code_item(by_idx["<clinit>"][1])
    check((regs, ins, outs, tries, debug_off) == (1, 0, 1, 0, 0),
          "<clinit> code: registers=1 ins=0 outs=1 tries=0 debug=0")
    check(walk_insns(insns) == [
        ("const-string", 0, S["owen_app"]),
        ("invoke@0x71", 1, m_idx(SYSTEM_DESC, "loadLibrary"), [0]),
        ("return-void",),
    ], "<clinit> insns: const-string v0 \"owen_app\"; "
      "invoke-static {v0} System.loadLibrary; return-void")

    # <init>
    regs, ins, outs, tries, debug_off, insns_size, insns = read_code_item(by_idx["<init>"][1])
    check((regs, ins, outs, tries, debug_off) == (1, 1, 1, 0, 0),
          "<init> code: registers=1 ins=1 outs=1 tries=0 debug=0")
    check(walk_insns(insns) == [
        ("invoke@0x70", 1, m_idx(SUPER_DESC, "<init>"), [0]),
        ("return-void",),
    ], "<init> insns: invoke-direct {p0} BroadcastReceiver.<init>; return-void")

    # onReceive
    regs, ins, outs, tries, debug_off, insns_size, insns = read_code_item(by_idx["onReceive"][1])
    check((regs, ins, outs, tries, debug_off) == (3, 3, 1, 0, 0),
          "onReceive code: registers=3 ins=3 outs=1 tries=0 debug=0")
    check(walk_insns(insns) == [
        ("invoke@0x71", 1, m_idx(CLASS_DESC, "nativeOnReceive"), [2]),
        ("return-void",),
    ], "onReceive insns: invoke-static {p2} nativeOnReceive(Intent); return-void")

    # ---- map list ----
    (n_map,) = struct.unpack_from("<I", data, map_off)
    map_items = []
    for i in range(n_map):
        item_type, _unused, size, off = struct.unpack_from("<HHII", data, map_off + 4 + 12 * i)
        map_items.append((item_type, size, off))
    offs = [m[2] for m in map_items]
    check(offs == sorted(offs) and len(set(offs)) == len(offs),
          "map_list sorted by offset (strictly)")
    check(map_items[0] == (TYPE_HEADER_ITEM, 1, 0), "map: header entry")
    for item_type, header_off, header_size_field in (
        (TYPE_STRING_ID_ITEM, string_ids_off, n_strings),
        (TYPE_TYPE_ID_ITEM, type_ids_off, n_types),
        (TYPE_PROTO_ID_ITEM, proto_ids_off, n_protos),
        (TYPE_METHOD_ID_ITEM, method_ids_off, n_methods),
        (TYPE_CLASS_DEF_ITEM, class_defs_off, 1),
    ):
        entry = next((m for m in map_items if m[0] == item_type), None)
        check(entry is not None and entry[1] == header_size_field and entry[2] == header_off,
              f"map: {item_type:#06x} entry matches header (size={header_size_field}, off={header_off:#x})")
    check(next(m for m in map_items if m[0] == TYPE_MAP_LIST) == (TYPE_MAP_LIST, 1, map_off),
          "map: map_list entry points at itself")
    for item_type in (TYPE_CODE_ITEM, TYPE_STRING_DATA_ITEM, TYPE_TYPE_LIST, TYPE_CLASS_DATA_ITEM):
        check(any(m[0] == item_type for m in map_items),
              f"map: contains {item_type:#06x} section")
    check(all(0 <= off < file_size for _, _, off in map_items if off),
          "map: every entry offset within the file")
    check(map_off + 4 + 12 * n_map == file_size, "map_list ends exactly at file_size")

    # every code_off seen in class_data is 4-aligned and in range
    for name, (flags, code_off) in by_idx.items():
        if code_off:
            check(code_off % 4 == 0 and code_off + 16 <= file_size,
                  f"code item of {name} at {code_off:#x} is 4-aligned and in range")

    # data section bookkeeping
    check(data_off % 4 == 0 and data_off + data_size == file_size,
          "data_off 4-aligned, data_off + data_size == file_size")

    return report


def main() -> int:
    default_out = Path(__file__).resolve().parents[2] / "assets" / "dex" / "classes.dex"
    out_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_out

    dex = build_dex()

    print(f"emitting {out_path} ({len(dex)} bytes)")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(dex)

    print("\n=== DEX self-check (re-parsing the emitted file) ===")
    try:
        report = validate(dex)
    except Fail as e:
        print(f"  FAIL: {e}")
        print("  validation FAILED — fix the emitter and re-run")
        return 1
    for status, msg in report:
        print(f"  [{status}] {msg}")
    n_pass = sum(1 for s, _ in report if s == "PASS")
    print(f"\n{n_pass}/{len(report)} checks passed — dex is well-formed.")
    print("head bytes:", dex[:72].hex(" "))
    return 0


if __name__ == "__main__":
    sys.exit(main())
