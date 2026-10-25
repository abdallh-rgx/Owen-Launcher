# RUNTIME SUPPORT v5 — محرك الإيجاد الغاشم وقت-التشغيل

## الفكرة
بدلاً من الاعتماد فقط على أنماط بايت ثابتة لكل إصدار (تتطلب تحليل كل ثنائي مسبقاً)،
يضيف v5 محرك **RT::** الذي يجد الدوال **على الجهاز بعد تحميل libUE4.so/libUnreal.so مباشرة** —
بالفحص الغاشم للذاكرة حيث الـ relocations مُطبَّقة والـ vtables مكتملة.

هذا يعني: أي إصدار بين 5.x و 30.00 — بما فيها الإصدارات الوسيطة غير المتاحة للتحليل
(6.31، 12.41، 20.10، 21.40، 22.00، 27.11 أندرويد...) — تُوجد فيه الدوال تلقائياً وقت التشغيل.

## المحرك (namespace RT)

| البدائية | الوصف |
|---|---|
| `Init()` | يقرأ ELF64 phdrs عند ModuleBase → حدود .text (X) و .data/.bss (W) |
| `FindWStr(FindAStr)` | مسح كل الوحدة عن نص UTF-16LE (literal pools داخل .text + .rodata) أو ASCII |
| `RefsTo(target)` | كل مراجع ARM64 للعنوان: ADRP+ADD، ADR مباشر (±1MB)، ADRP+LDR عبر GOT (يقرى الـ slot المُحوَّل) |
| `Deoutline(addr)` | يتبع سلاسل `B imm` (stub→thunk→الأصل) حتى 4 قفزات |
| `BackscanPrologue(addr)` | يمسح للخلف حتى 0x900 بايت عن STP X29,X30,[SP,#-N]! / SUB SP / PACIASP ويتوقف عند RET |
| `StringAnchorFunc(strs)` | السلسلة الكاملة: نص → مرجع → de-outline → بداية الدالة |
| `VtSlotsWith(func)` | يمسح الذاكرة القابلة للكتابة عن كل slot يساوي عنوان الدالة (جداول vtable بعد الـ relocation) |
| `VtNeighbor(anchor, delta)` | الجار في نفس الـ vtable بإزاحة ±delta — **تصويت أغلبية عبر كل الجداول المطابقة** |
| `VtMethodAt(anchor, index)` | قراءة vtable[index] عبر جدول الـ anchor |
| `VtIndexOf(func)` | حساب فهرس الدالة في جدولها (لدوال *Vft) |
| `FindNativeThunk(name)` | مسح .data عن أزواج `{char* Name, uintptr Thunk}` — جدول FNativeFunctionRegistrar بعد الربط |
| `NativeOrig(name)` | orig-climb: الـ thunk → مسح 0x140 بايت عن BL → التنفيذ الأصلي |
| `GIsFromAnchor(skip)` | نقل حرفي لمنطق Erbium الأصلي (Finders.cpp) إلى ARM64: مسح رجعي 0x100 بايت من مرجع `AllowCommandletRendering` عن LDRB/STRB عالمية (مباشرة أو عبر GOT) — skip=1 → GIsServer, skip=2 → GIsClient |

## التحقق (على هذا الجهاز)
- **بناء ناجح**: الملف كاملاً (250KB) يُترجم بـ g++ -std=c++17 بلا أي خطأ
- **19/19 اختبار وظيفي ناجح** على وحدة ARM64 اصطناعية (mmap + ELF phdrs حقيقية + تعليمات ADRP/ADD/ADR/LDR/LDRB/B/STP مُرمَّزة يدوياً):
  - FindWStr/FindAStr، RefsTo بالآليات الثلاث، Deoutline، BackscanPrologue،
    StringAnchorFunc، VtSlotsWith/VtNeighbor/VtMethodAt/VtIndexOf، FindNativeThunk،
    GIsFromAnchor (بالضبط دلالات skip في Erbium الأصلي)
  - الاختبار: `tools/rt_test.cpp` (يعاد تشغيله بأمر g++ بسيط)

## علاقات vtable المستخدمة (الأدلة)

| العلاقة | الأدلة |
|---|---|
| IsNetReady = FlushDormancy − 17 | 11/11 نسخة أندرويد (توثيق v3) + ويندوز 27.11: كل جداول NetConnection الخمسة تتفق (0x6fff104) |
| StartBecomingDormant = CloseActorChannel + 11 | 10 نسخ أندرويد (توثيق v3) |
| InitListen = TickFlush − 15 | 6 نسخ أندرويد (توثيق v3) |
| SendClientAdjustment = Reset + 270/+276 | ويندوز 21.30 (+270 في جدولين) / ويندوز 27.11 (+276 في 4 جداول) — متعاقبة بالإصدار (clustered ≥23.00) |
| CanAffordToPlaceBuildable = SendClientAdjustment + 361/+373 | ويندوز 21.30 (936−575 في 3 جداول) / 27.11 (PayBuildable−SCA=385 و −12) |
| PayBuildable = CanAfford + 12 | ويندوز 21.30 (948−936 في 3 جداول متوافقة) |

**لماذا delta وليس فهرس مطلق؟** الفهارس المطلقة من مسح ويندوز تتضخع بدمج الجداول المتجاورة
(مثال مُقاس: OnItemInstanceAdded ظهر 183 بالمسح مقابل 143 الحقيقي على 21.30).
الـ deltas داخل نفس الجدول محصَّنة ضد الدمج — وهي متطابقة عبر ABIs (مُثبت).

## سلاسل الإيجاد لكل دالة (ترتيب المحاولة)
1. الأنماط الثابتة v4 (717 نمط فريد مُتحقق — تعمل أولاً وأسرع)
2. native-pair: `FindNativeThunk`/`NativeOrig` (كل الإصدارات لها جدول natives)
3. string-anchor: النصوص المميزة (FlushDormancy, UpdateSafeZonesPhase, PreSendUpdate, ...)
4. vtable-delta: العلاقات أعلاه مع anchors تعمل 11/11 (GetPlayerViewPoint, TickFlush, ...)
5. GIs/Globals: `GIsFromAnchor` (يحل أيضاً تعارضات النسخ القديمة 5.41-18.40/23.50)

## ما زال نمطاً فقط (66 خلية / 18 دالة)
معظمها دوال بلا نصوص ولا natives ولا علاقات vtable معروفة:
`CallPreReplication` (4/11), `ClientHasInitializedLevelFor` (1/11), `GetWorldContext` (3/11),
`CreateNetDriverWorldContext` (5/11), `AttemptDeriveFromURL` (1/11), `GetSparseClassData` (1/11),
`ServerReplicateActors` (7/11), `GetNamePool` (8/11), `ConstructAbilitySpec` (9/11), وغيرها
(معظم القائمة لديها تغطية ثابتة 8-11/11 أصلاً).
لهذه: أنماط v4 + مسار SDK الانعكاسي في Erbium (DefaultObjImpl → Vft scan) وقت التشغيل.

## النتيجة النهائية
- **913/979 خلية (93.3%)** مع تغطية فعلية عند التشغيل (نمط فريد أو سلسلة RT::)
  مقارنة بـ 759/979 (78%) أنماطاً فقط في v4
- **GIs**: اكتشاف وقت-تشغيل يحل الـ 37 خلية المتعارضة نهائياً (المنطق الأصلي نفسه)
