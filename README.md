# Erbium Android — قاعدة بيانات دوال فورتنايت أندرويد الكاملة (5.41 → 30.00)

**Fortnite Android ARM64 Complete Functions Database + Erbium Finders Port**

هذا المستودع يحتوي على **كل الدوال الـ Native (UFunction)** المستخرجة ثابتاً (Static Extraction) من ملفات اللعبة الأصلية `libUnreal.so` / `libUE4.so` لـ **11 نسخة أندرويد** تغطي النطاق المطلوب كاملاً من **5.41 إلى 30.00** — ليس فقط دوال Erbium، بل **كل ما يقع في النطاق**: 49,859 اسم دالة فريد.

---

## 📊 الإحصائيات

| النسخة | عدد الدوال | ملف قاعدة البيانات |
|--------|-----------:|--------------------|
| 5.41  | 10,710 | `natives-db/natives-v5.41.json` |
| 7.40  | 12,306 | `natives-db/natives-v7.40.json` |
| 9.41  | 15,846 | `natives-db/natives-v9.41.json` |
| 13.40 | 19,927 | `natives-db/natives-v13.40.json` |
| 17.50 | 22,824 | `natives-db/natives-v17.50.json` |
| 18.40 | 23,198 | `natives-db/natives-v18.40.json` |
| 19.10 | 24,736 | `natives-db/natives-v19.10.json` |
| 21.30 | 26,347 | `natives-db/natives-v21.30.json` |
| 22.40 | 27,437 | `natives-db/natives-v22.40.json` |
| 23.50 | 29,396 | `natives-db/natives-v23.50.json` |
| 30.00 | 39,145 | `natives-db/natives-v30.00.json` |

- **مجموع الأسماء الفريدة: 49,859 دالة**
- **دوال موجودة في كل النسخ (5.41→30.00): 8,515 دالة**
- العناوين بصيغة **RVA**: `runtime_address = module_base + RVA`

## 📁 محتويات المستودع

```
├── natives-db/                  قاعدة بيانات كل الدوال per-version
│   ├── natives-vX.YY.json       {اسم الدالة: {thunk, orig}} لكل نسخة
│   ├── by-name.json             فهرس عكسي: اسم الدالة → كل النسخ وعناوينها
│   ├── version-diffs.json       الفروق بين كل نسختين متتاليتين (مضاف/محذوف)
│   ├── erbium-native-map.json   ربط دوال Erbium بمقابلاتها الـ native
│   └── SUMMARY.md               ملخص القاعدة
├── offsets-headers/             نفس البيانات بصيغة #define جاهزة للـ #include
│   └── offsets-vX.YY.h          (uefinder v6/v7 headers)
├── erbium-finders/              منفذ Erbium لأندرويد ARM64
│   ├── Finders-Android-FULL.cpp     47 دالة × 11 نسخة (396 نمطبايت فريد)
│   ├── COVERAGE-MATRIX.md           مصفوفة التغطية لكل دالة/نسخة
│   ├── FINAL-REPORT.md              التقرير النهائي + المنهجية
│   ├── VERIFICATION_REPORT.md       تقرير التحقق الأولي (تواقيعك الثلاثة)
│   ├── FortVehicleSeatWeaponComponent-FIXED.cpp
│   └── Finders-Android.cpp          النسخة المبكرة
└── tools/                       أدوات الاستخراج (يمكن تشغيلها على أي نسخة جديدة)
    ├── uefinder_ext.c               المستخرج الثابت (v6/v7) — ينتج offsets.h
    ├── uetool.py                    محلل ARM64 (capstone/numpy)
    └── mega/                        خط الأنابيب الكامل (أنماط، مطابقة عابرة للمعماريات)
```

## 🔧 كيف تستخدم البيانات

### 1) هوك مباشر بالعنوان (لنسخة محددة)
```cpp
// مثال: ServerHandlePickup على 21.30
uintptr_t base = (uintptr_t)dlopen("libUnreal.so", RTLD_NOLOAD); // أو GetModuleHandle المكافئ
auto addr = base + 0x70AB21C;  // THUNK من natives-v21.30.json
// هوك العنوان مباشرة (thumb غير موجود — ARM64 نظيف)
```

### 2) البحث بالاسم عبر النسخ كلها (مستقل عن النسخة)
```cpp
// runtime: ابحث عن UFunction بالاسم ثم خذ حقل Func (native thunk)
UFunction* Func = UObject::FindObject<UFunction>("Function /Script/FortniteGame.FortPlayerController.ServerHandlePickup");
void* NativeThunk = Func->Func;  // نفس عنوان الـ thunk في القاعدة
```
> هذه الطريقة تعمل عبر كل النسخ بلا أنماط إطلاقاً — القاعدة تخدم للتحقق والـ impl الأصلي.

### 3) دوال Erbium (الأنماط)
افتح `erbium-finders/Finders-Android-FULL.cpp` — كل دالة لها فروع `if (VersionInfo.FortniteVersion >= X)` بنمط ARM64 مطابق مرة واحدة بالضبط في ثنائية نسخته (396/396 VERIFIED-UNIQUE).

## ⚠️ ملاحظات تقنية مهمة

1. **thunk مقابل orig**:
   - `thunk` = دالة `execXXX` الخاصة بالـ UFunction — نقطة الـ ExecHook المثالية.
   - `orig` = التنفيذ الأصلي المستقل عندما يوجد (بعض الـ impl مطوية بـ ICF على UE5 أندرويد: `EquipVehicleWeapon`, `Reset`, `SendClientAdjustment` — لهوكها استخدم الـ thunk/UFunction).
2. **التحقق من الجودة**: 20,965/20,965 (100%) من عناوين 21.30 مطابقة لاستخراج مستقل بمنهجية مختلفة.
3. **الاستخراج ثابت بالكامل** — من جداول `FNativeFunctionRegistrar` داخل الثنائي، بدون تشغيل اللعبة، لذا يعمل على Shipping builds.
4. النسخ القديمة (5.41–9.41) تحتوي دوال أقل لأن اللعبة نفسها كانت أصغر (UE4 مبكر).

## 🎯 ما الذي يغطيه هذا المستودع مقابل Erbium الأصلي

| Erbium (Windows x64) | هذا المستودع (Android ARM64) |
|---|---|
| 83 دالة Find_* بأنماط x64 لنسخ محددة | 49,859 دالة مسماة بعناوين مباشرة × 11 نسخة |
| يعمل على النسخ المدعومة في شجرة الفروع | يغطي 5.41 → 30.00 بلا فجوات (11 نسخة مرجعية) |
| يتطلب تشغيل اللعبة (Memcury) | استخراج ثابت + تحقق آلي |

النسخ المرجعية الـ 11: 5.41, 7.40, 9.41, 13.40, 17.50, 18.40, 19.10, 21.30, 22.40, 23.50, 30.00 — كل دالة بين نسختين متتاليتين يمكن استخراجها بنفس الأداة (`tools/uefinder_ext.c`) على ثنائية النسخة المطلوبة.

## 📜 المنهجية (ملخص)

1. تحليل ELF لملفات `.so` (أقسام + relocations).
2. تحديد جداول تسجيل الـ natives: أزواج (مؤشر نص الاسم ← مؤشر الدالة) في `.data.rel.ro` مع relocations.
3. تحليل السلاسل + التحقق من صيغة المعرفات → استخراج `{اسم: {thunk, orig}}`.
4. تتبع الـ thunk → الـ impl الحقيقي (مع معالجة الـ thunks المقتطعة out-lined والـ ICF).
5. تحقق مستقل بماسح ثانٍ + مقارنة مع مصادر خارجية (100% تطابق على 21.30).

التفاصيل الكاملة في `erbium-finders/FINAL-REPORT.md` و `tools/mega/`.

---
**تنبيه**: هذا العمل لأغراض البحث والتعليم (private server research). 
