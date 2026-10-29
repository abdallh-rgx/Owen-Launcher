# Owen Launcher — T6-FINAL build report

Append-only log of the final build pass (JNI fix → handcrafted dex → APK).

## 0. Starting point (verified)

- `cargo check -p owen-app --target aarch64-linux-android`: 79 errors, all in
  `crates/owen-app/src/jni_android.rs` (E0277 ×64, E0308 ×7, E0106 ×4, E0596,
  E0507, E0061, E0015) — jni-0.21.1 API mismatches.
- `assets/dex/` empty (no classes.dex).
- `dist/OwenLauncher-v1.8.0-release-stub.apk` exists (stub .so, no dex, 214 entries).

(sections appended as work progresses)

## 1. jni_android.rs — FIXED (0 errors)

`cargo check -p owen-app --target aarch64-linux-android`: **Finished, 0 errors**
(3 pre-existing warnings only — unused imports in lib.rs/platform.rs).

Changes (design, function signatures, public API and all other files untouched):

- **jni-0.21 lifetimes**: every private helper now takes `JNIEnv<'_>` (or a
  named `'local` when it returns a `JObject` tied to the env frame:
  `package_manager`, `make_uri`, `new_action_intent`, `parcelable_extra`,
  `session_obj`), matching `call_method`/`get_static_field` returning
  `JValueOwned<'local>`.
- **Error plumbing**: new private `enum JniErr { Jni(jni::errors::Error), Msg(String) }`
  with `From<Error>/From<String>/From<&str>` + `Display`. `with_env`/`jni_run`
  closures now return `Result<T, JniErr>`; `jni_run` still returns
  `Result<T, String>` so every caller is unchanged. `?` works on both jni
  errors and the String helpers; closure tails returning `Result<_, String>`
  got `.map_err(JniErr::from)` (open_url / both settings helpers / launch_game).
- **WAITING static**: `Mutex::new(HashMap::new())` is not const →
  `OnceLock<Mutex<HashMap<…>>>` + `fn waiting()` accessor (same pattern as the
  existing `bus()`); all 4 lock sites updated; `on_result` guard now `mut`.
- **`call_method` results**: `parcelable_extra` no longer wraps into `Option`
  then pattern-matches `Ok(..)` — `typed`/`legacy` stay `Result<JValueOwned>`
  and are matched with `Ok(v)` / `match legacy { Ok(v)…, Err(_) }`.
- **`JString::from_raw(obj.into_raw())`** (moved out of a shared ref) →
  `JString::from_raw(obj.as_raw())` (borrows the raw handle; jni-0.21 wrappers
  never delete local refs on drop).
- **`get_byte_array_region`**: jni-0.21 signature is `(array, start, buf)`
  (buf length carries the count) — dropped the extra `n as jint` argument.
- **`register_native_methods` / `new_global_ref`** closure tails:
  `.map_err(JniErr::from)`; the null-Application branch of `context_global`
  wraps in `JniErr::Jni(Error::JniCall(JniError::InvalidArguments))` (same
  failure it signaled before).
- `LIB_NAME` const: added `#[allow(dead_code)]` (it documents the
  `System.loadLibrary("owen_app")` contract with the dex; referenced by docs,
  not code — was dead in the original too, just never type-checked).

`cargo build --release -p owen-app --target aarch64-linux-android`:
**Finished `release`** → `target/aarch64-linux-android/release/libowen_app.so`
(8,432,720 bytes, hard-linked as the cdylib output).
`file`: *ELF 64-bit LSB shared object, ARM aarch64, … for Android 24, NDK r27c*.
`nm -D` shows **both** `ANativeActivity_onCreate` (from android-activity) and
`JNI_OnLoad` (from jni_android) as `T` (defined, global).

## 2. assets/dex/classes.dex — HANDCRAFTED (tools/dex/make_dex.py)

New deterministic emitter checked in at `tools/dex/make_dex.py` (emitter +
mandatory from-scratch re-parser/validator in one script). Output:
`assets/dex/classes.dex`, **992 bytes**, DEX version 035, one class:

- `com.abdallhrgx.owenlauncher.core.patch.InstallResultReceiver extends
  android.content.BroadcastReceiver`, access 0x1.
- `<clinit>()V` flags 0x10009 — `const-string v0, "owen_app"`;
  `invoke-static {v0}, Ljava/lang/System;->loadLibrary(Ljava/lang/String;)V`;
  `return-void` (registers 1 / ins 0 / outs 1, insns 6 code units:
  `1a 00 | 12 00 | 71 10 | 05 00 | 00 00 | 0e 00`).
- `<init>()V` flags 0x10001 — `invoke-direct {p0},
  Landroid/content/BroadcastReceiver;-><init>()V`; `return-void`
  (registers 1 / ins 1 / outs 1).
- `onReceive(Landroid/content/Context;Landroid/content/Intent;)V` flags 0x1 —
  `invoke-static {p2}, …InstallResultReceiver;->nativeOnReceive(Landroid/content/Intent;)V`;
  `return-void` (registers 3 / ins 3 / outs 1: p0=this, p1=Context, p2=Intent).
- `nativeOnReceive(Landroid/content/Intent;)V` flags 0x109 (PUBLIC|STATIC|NATIVE),
  declared with code_off = 0.

Table layout: header(0x70) → string_ids 19 @0x70 → type_ids 7 @0xBC →
proto_ids 4 @0xD8 → method_ids 6 @0x108 → class_def 1 @0x138 → code items 3
@0x158/0x174/0x18C → string_data 19 @0x1A4 → type_lists 3 @0x328 (each padded
to 4) → class_data 1 @0x340 → map_list 11 entries @0x358 → EOF 0x3E0.
Strings sorted by UTF-16 code-unit order (all ASCII); type_ids sorted by
string idx; proto shortys V/VLL/VL/VL verified against param lists; method_ids
sorted (class_idx, name_idx, proto_idx); map entries sorted by offset;
checksum adler32(bytes[12:]) = 0x1543afd1; signature sha1(bytes[32:]).

**Self-check: 128/128 PASS** (header, checksum+signature recompute, all
string/type/proto/method ids, class_def, class_data uleb walk, all three code
items decoded opcode-by-opcode — const-string 0x1a, invoke-direct 0x70,
invoke-static 0x71, return-void 0x0e with exact registers/method indices —
and the full map_list cross-checked against the header offsets).

### Deviations from the task's dex spec (all deliberate, ART-correctness):

1. `<clinit>` access flags **0x10009**, not "0x10001": the task's own
   convention note names PUBLIC|STATIC|CONSTRUCTOR = 0x10009; ART's dex
   verifier requires `<clinit>` to carry ACC_STATIC (0x10001 would fail
   `DexFileVerifier` on device). 0x10001 kept for `<init>` as specified.
2. **direct_methods_size = 3, not 2**: `nativeOnReceive` must be *declared* in
   class_data (code_off = 0) or the method does not exist — `onReceive`'s
   `invoke-static …nativeOnReceive` would throw NoSuchMethodError and
   JNI_OnLoad's RegisterNatives would find nothing to register.
3. **onReceive ins_size = 3, not 2**: dex `ins_size` counts `this` for
   instance methods (this+Context+Intent); ART computes the incoming-register
   count from the signature and rejects the method otherwise.
4. **method_ids emitted SORTED** (dx/d8-compatible order by
   class_idx/name_idx/proto_idx). "Not sorted" is allowed by the spec, but
   sorted matches every real-world dex and is strictly safer.
5. map_list includes **TYPE_TYPE_LIST (0x1001)** and **TYPE_CLASS_DATA_ITEM
   (0x2000)** entries in addition to the ones the task listed — those
   sections exist (proto params + the class body), and ART cross-checks the
   map against the file; omitting them risks "inconsistent map" failures.
