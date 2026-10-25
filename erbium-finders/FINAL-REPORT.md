# التقرير النهائي الشامل — منفذ Erbium لأندرويد ARM64
## إيجاد كل دوال Erbium وبناء باترينات تعمل من 5.41 إلى 30.00

**المدخلات**: 11 ثنائي أندرويد (5.41, 7.40, 9.41, 13.40, 17.50, 18.40, 19.10, 21.30, 22.40, 23.50, 30.00) + FortniteClient-Win64-Shipping.exe (21.30) + كامل مصدر Erbium

**المخرجات**:
- `Finders-Android-FULL.cpp` — كل الدوال بأنماط ARM64 بفروع إصدارات (بنمط Erbium تماماً، متوافقة مع Memcury مباشرة)
- `COVERAGE-MATRIX.md` — مصفوفة التغطية (دالة × نسخة)
- `final_resolved.json` — كل العناوين والأنماط مع حالة التحقق الآلي لكل خلية

---

## 1) المنهجية (خط الإنتاج الآلي)

| المرحلة | الآلية | الدوال |
|---|---|---|
| wave1 | مراسي نصية: نص → ADRP+ADD refs → الدالة الحاوية | 46 دالة × 11 نسخة |
| natives | جدول الـ natives على exe ويندوز ({اسم→مؤشر} في .rdata): 20965 مقترنة، 3484 غير ملتبسة | بذور للمطابقة |
| x2a | مطابقة عرضية: باترينات x64 على exe → ميزات → مطابقة بيانية (جيران+أحجام+ثوابت) | الدوال عديمة النصوص |
| vtable | مرساة جداول الفيرتشوالز: ترتيب الدوال الافتراضية متطابق بين ABIs — جار متطابق يعطي الإزاحة | SetWorld, GetWorldContext, SRA |
| a2a | انتشار عبر النسخ: بذور natives (30K+) بين النسخ + مطابقة بيانية | ملء فجوة 30.00 (فقدت نصوصها) |
| wave3 | خواص خاصة: SRA عبر تحليل BLR في TickFlush، vtable slot-1 لـ GetPlayerViewPoint | |

**التحقق الآلي**: كل نمط يُعاد فحصه آلياً ضد .text كاملاً — يجب أن يطابق **مرة واحدة بالضبط** في العنوان المتوقع. نتائج التحقق في `verify` لكل خلية.

**التحقق المستقل**: وكيل تحقق خارجي أعاد فحص عينة عشوائية (25 نمطاً، 19 دالة، 10 نسخ) بماسح مستقل كُتب من الصفر: **25/25 نجاح ميكانيكي** (فريد + العنوان صحيح).

---

## 2) الاكتشافات الحرجة (أهم من الأنماط نفسها!)

### أ) الاقتطاع (Machine Outlining) على 13.40+
clang على أندرويد يقتطع تسلسلات السجل في دوال مُصغّرة (0x24-0x2C بايت) ثم thunk تفرّع (4 بايت `B`). **أي مرساة نصية تعطي الـ stub وليس الدالة الحقيقية!**
- الحل المطبق: تتبع المتصلين عبر الطبقتين (stub ← thunk ← الدالة الحقيقية)
- أُصلح **169 نتيجة** كانت خاطئة
- القاعدة الذهبية: أي عنوان بدالة < 0x60 بايت مرتبطة بنص = مقتطعة → اتبع المتصلين
- استثناء: دالة صغيرة **بلا** مراجع نصوص + منطق صرف (مثل GetNetMode 0x24) = حقيقية — لا تتبعها!

### ب) ICF Folding على UE5 أندرويد (19.10+)
دوال كاملة تُطوى إلى `ret` واحد مشترك بين آلاف الدوال:
- **EquipVehicleWeapon** (مؤكد سابقاً — 2605 دالة تشترك في العنوان!)
- **Reset** — stub 4 بايت
- **SendClientAdjustment** — مرشحها 24 بايت (مشبوه)
- **الحل**: Hooking::ExecHook على UFunction المقابلة بدلاً من هوك العنوان

### ج) دوال السجل المُقتطعة تحتاج معالجة في هوكاتك
إذا كنت ستنقل دوالاً نصية المراسى بنفسك — طبّق de-outline دائماً.

### د) فقدان النصوص في 30.00
UE5.4 غيّر التسجيل — 36 مرساة نصية اختفت. حُلّت بالمطابقة البيانية (natives-seeded graph matching) من 23.50.

---

## 3) التغطية النهائية

انظر `COVERAGE-MATRIX.md` للتفصيل الكامل. ملخص التقريبات:

- **دوال مغطاة بالكامل تقريباً (11/11)**: GetNetMode, PickTeam, HandleMatchHasStarted, EnterAircraft, KickPlayer, GiveAbilityAndActivateOnce, StartAircraftPhase, SetPickupTarget, GiveAbility (بعد الإصلاح), SetState, StreamInMyBuilding, InitializePlayerGameplayAbilities, InternalTryActivateAbility, ShouldAllowServerSpawnDecoVft, SendRequestNow, FinishWorldInitialization, CanAffordToPlaceBuildableClass, ApplyCharacterCustomization (10/11), IsNetRelevantFor (10/11)...
- **كومة النسخ (13.40-30.00)**: TickFlush, ReplicateActor, ServerReplicateActors, ReplaceBuildingActor, InitializeBuildingActor(+next), CreateChannel, GameSessionPatch, ActivatePhase, LoadPlayset, SendDestructionInfo, PreSendUpdate (30.00 فقط — Iris)...
- **21.30 فقط (عبر المطابقة العرضية)**: CallPreReplication, GetWorldContext, SetWorld, FinishedTargetSpline — قابلة للتوسيع بتشغيل نفس الأدوات
- **النسخ القديمة (5.41-9.41)**: مجموعة المراسي النصية المتاحة (~22-26 دالة) — الفجوة بلا natives table هناك

### الخلايا المميزة بعلامات خاصة
- `ICF` — الدالة مطوية على تلك النسخة (استخدم ExecHook)
- `~✓` — نمط موجود لكن غير فريد تماماً (متعدد المطابقات — تجنبه أو وسّعه يدوياً)
- `·` — غير محلولة على تلك النسخة (انظر §5 لطرق إكمالها)

---

## 4) إصلاحات على ملفاتك الثلاثة السابقة
(من الجلسة السابقة — ما زالت سارية)
1. EquipVehicleWeapon على UE5 → ExecHook (ICF)
2. OnPawnEnterSeat 30.00 → النمط المصحح (الهدف 0xA160AAC)
3. GetNetMode الآن محلولة بشكل مؤكد 11/11 نسخة بالشكل الدلالي (LDR [x0+0x38] + return 3 + World [x0+0xE0..0x108])

---

## 5) كيف تُكمل الفجوات المتبقية

كل الأدوات في `/home/z/re-analysis/mega/`:
- `engine.py` — السياق + مصنع الأنماط (VersionCtx(version).gen_pattern(addr))
- `anchors.py` — البحث النصي (anchored_funcs_v2)
- `x64tool.py` + `natives_x64.py` — الجانب المقابل لويندوز
- `wave2b.py` — المطابق البياني (Matcher)
- `vtable_x64.py` — مرساة الجداول الافتراضية
- `a2a_v2.py` — الانتشار بين النسخ
- `deoutline.py` — إزالة الاقتطاع (استخدمه دائماً!)

لأي دالة/نسخة ناقصة: (1) جرب مرساة نصية، (2) ثم de-outline، (3) ثم natives pairing إن وُجد exe مقابل، (4) ثم المطابقة البيانية.

---

## 6) كيفية الاستخدام في Erbium

```cpp
// استبدل Finders.cpp كلاً بدالة بهذا الشكل (مولّدة جاهزة في Finders-Android-FULL.cpp):
uint64_t FindPickTeam()
{
    static uint64_t PickTeam = 0;
    static bool bInit = false;
    if (!bInit) {
        bInit = true;
        float v = VersionInfo.FortniteVersion; // من ++Fortnite+Release-X.YY في .rodata
        if (v >= 30.00f)      PickTeam = Memcury::Scanner::FindPattern("...").Get();
        else if (v >= 23.50f) PickTeam = Memcury::Scanner::FindPattern("...").Get();
        ...
        else if (v >= 5.41f)  PickTeam = Memcury::Scanner::FindPattern("...").Get();
    }
    return PickTeam;
}
```

ملاحظات:
- بعض الأنماط تطابق `func+offset` — المولّد يضيف `+ 0xNN` تلقائياً في السطر
- كشف الإصدار: النص `++Fortnite+Release-X.YY` موجود في .rodata بترميز UTF-16 (كود جاهز في رأس الملف المولّد)
- الأساسيات (GObjects/GNames/ProcessEvent/SpawnActor/StaticFindObject) في `Finders-Android.cpp` السابق (الباحثات الدلالية) — ما زالت صالحة وتتكامل مع هذا الملف

---

## 7) حدود المسؤولية
- كل خلية "VERIFIED-UNIQUE" = النمط يطابق مرة واحدة بالضبط في ذلك الثنائي تحديداً (نفس مبدأ Erbium على ويندوز — بناء مختلف لنفس الإصدار قد يحتاج إعادة توليد)
- الخلايا بغير تحقق فريد معلنة صراحة
- الدلالة (أن هذه هي الدالة المقصودة) مبنية على: مراسٍ نصية/بيانية/جداول + تدقيق خارجي بالعينة — الدوال المعلّمة "manual-review" تحتاج نظرة أخيرة
