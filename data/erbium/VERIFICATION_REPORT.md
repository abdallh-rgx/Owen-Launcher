# تقرير التحقق الشامل — Erbium Android Port
## (تحليل ثابت عبر 11 نسخة أندرويد + مطابقة عابرة للمعماريات مع Windows 21.30)

**الأدوات المستخدمة**: uefinder v6 (المستخدم) + uefinder_ext (--dump mode أضفته) + محلل ARM64 دلالي (Python/capstone) + أنماط Erbium x64 على FortniteClient-Win64-Shipping.exe

---

## 1) التحقق من تواقيعك اليدوية

### OnPawnEnterSeat — عبر تتبع سلسلة thunk→impl من جدول الـ natives
| النسخة | عنوانك | العنوان الحقيقي (من الـ thunk) | النتيجة |
|---|---|---|---|
| 13.40 | (لا نمط) | ليست native | ✗ لا يوجد فرع — hook سيفشل بصمت |
| 17.50 | 0x89BE690 | — (ليست native على UE4) | ✓ مطابقة فريدة (1 hit) |
| 18.40 | 0x786D724 | — | ✓ مطابقة فريدة |
| 19.10 | 0x7B19D0C | 0x7B19D0C (impl من thunk 0x7B2524C) | ✓✓✓ **مؤكد بالسلسلة** |
| 21.30 | 0x48ADC9C | 0x48ADC9C | ✓✓✓ **مؤكد** |
| 22.40 | 0x4A94810 | 0x4A94810 | ✓✓✓ **مؤكد** |
| 23.50 | 0x46F0184 | 0x46F0184 | ✓✓✓ **مؤكد** |
| **30.00** | 0xDF8EA64 | **0xA160AAC** (impl مُفصّل بعد الـ thunk مباشرة) | ✗✗✗ **خطأ — نمطك يمسك دالة أخرى!** |

**التصحيح لـ 30.00** (النمط الحقيقي المستخرج من 0xA160AAC):
```
"FF 43 01 D1 FD 7B 02 A9 FD 83 00 91 F6 57 03 A9 F4 4F 04 A9 ?? ?? ?? ?? F4 03 03 2A F6 03 02 AA F5 03 01 AA"
```

### UnEquipVehicleWeapon — كلها حقيقية ✓
| النسخة | عنوانك | الحجم | مستدعون | الحكم |
|---|---|---|---|---|
| 13.40 | 0xA6DA880 | 0x374 | 2 | ✓ دالة حقيقية |
| 17.50 | 0xCE03858 | 0x394 | 2 | ✓ |
| 18.40 | 0xC0E7018 | 0x394 | 2 | ✓ |
| 19.10 | 0x9C9F8A4 | 0x38C | 2 | ✓ |
| 21.30 | 0x6A1C864 | 0x47C | 2 | ✓ |
| 22.40 | 0x7AF1F48 | 0x44C | 2 | ✓ |
| 23.50 | 0x8C57EB8 | 0x484 | 2 | ✓ |
| 30.00 | 0xCC32530 | 0x470 | 4 | ✓ |

### EquipVehicleWeapon — ⚠️⚠️⚠️ مشكلة بنيوية جذرية على UE5
**الاكتشاف الحاسم**: على UE5 أندرويد (19.10+)، الـ impl هي `ret` واحد (4 بايت) **مطوي بـ ICF ومشترك بين 2,605 دالة stripped** (تحققت: 3,225 موقع BL من 2,605 دوال مختلفة مثل GetDeathInfo/SafeTeleport/SetFuelUsageInfo كلها تشترك في نفس العنوان!).

| النسخة | حالة نمطك | الحقيقة |
|---|---|---|
| 17.50/18.40 | 51 مطابقة (عام جداً) | غير مهم — تستخدم مسار OnPawnEnterSeat |
| 19.10 | 51 مطابقة، أولها خطأ | الـ impl = ret مطوي ICF |
| **21.30** | 25 مطابقة، أولها 0x356AF08 **خطأ** | الـ impl الحقيقي = 0x2FB593C (ret مطوي مشترك!) |
| 22.40 | مطابقة واحدة = 0x5D52E0C = **الـ thunk وليس الـ impl** | خطر: هوك الـ thunk يعمل جزئياً فقط |
| 23.50 | 51 مطابقة | ret مطوي |
| 30.00 | 1 مطابقة = 0xCC3E134 = دالة صنف **آخر** (EquipVehicleWeapon_2) | خطر: هوك خاطئ |

**الحل الصحيح لـ UE5 أندرويد**: لا ت هوك الـ impl إطلاقاً — استخدم `Hooking::ExecHook` على UFunction "EquipVehicleWeapon" (نفس أسلوب ServerHandlePickup عندك). الـ thunk فريد لكل native والعناوين جاهزة عندك أصلاً في offsets.h!

---

## 2) الـ Offsets الأساسية الجديدة (كلها عبر بصمات دلالية مُتحقق منها)

### GObjects — بصمة المصفوفة المقسمة `LSR#13 + AND#0x7FFF8 + AND#0xFFFF + MOV/ORR#0x18 + MADD`
| النسخة | GObjects (RVA) | التخطيط | ملاحظات |
|---|---|---|---|
| 5.41 | — | Unchunked (UE4.19/4.20) | يحتاج بصمة مختلفة |
| 7.40 | — | Chunked 0x10400 (UE4.21) | قسمة غير قوة-2 (UMULH) |
| 9.41 | **0xCDB9790** | A: Objects@+0x10, Num@+0x24, Item=0x18 | |
| 13.40 | **0x12B1CD78** | A (16 صوت) | |
| 17.50 | **0x13724870** | A (8 أصوات) | |
| 18.40 | **0x11550ED0** | A | |
| 19.10 | **0x10398210** | A | |
| **21.30** | **0xDCD6E08** | A | ✓✓✓ **مطابق للـ MobileDumper بالضبط** |
| 22.40 | **0xE080FD8** | A (6 أصوات) | |
| 23.50 | **0xDD846D8** | A | |
| 30.00 | **0x13BA3C10** | **B: مؤشر!** deref ثم +0x10، **Item=0x14** | تغيرت البنية في UE5.4+ |

### ProcessEvent — بصمة دلالية (فحص الإشارات + FUNC_Native `ldrb[x1,#0xB1]+tbnz#2` + عضوية vtable)
| النسخة | PE (RVA) | التحقق |
|---|---|---|
| 5.41-9.41 | — | بنية أقدم (تحتاج score-scan مثل Dumper-7 وقت التشغيل) |
| 13.40 | **0xBCF7568** | فريدة بين دوال vtable المشتركة (6425 فتحة) |
| 17.50 | **0xC5B7178** | فريدة (7812 فتحة) + تقرأ GObjects+0x10 ✓ |
| 18.40 | **0xAD74BC8** | ✓ |
| 19.10 | **0xA894EF0** | ✓ |
| **21.30** | **0x7611B5C** | ✓✓✓ **مطابق للـ MobileDumper + VT slot 78** |
| 22.40 | **0x7F9CD20** | ✓ |
| 23.50 | **0x6115084** | ✓ |
| 30.00 | **0x9412930** | ✓✓ بنية كاملة مؤكدة (flags bit30 + FUNC_Native@0xB1 + Script.Num@0x68) |

### أخرى
- **GNames**: وضع `namepool` عندك يعمل — 21.30: 0xDC960C0 ✓✓✓ مطابق تماماً (BlocksBit 0x10, Stride 4, Blocks@+0x40)
- **GIsClient/GIsServer**: وضع `gis` يعمل — 21.30: 0xDA25050
- **FName::ToString(FString&)**: 21.30 = **0x74937CC** (عبر سلسلة GetObjectName native → BL → decode البلا: >>13 &0x7FFF8 + Blocks[+0x40] + stride4 + wide 0x40) — الطريقة تتكرر على كل النسخ عبر الـ offsets.h
- **SpawnActor** (مرسا STAT_SpawnActorTime): 13.40: 0xC07D830 | 17.50: 0xC2A20A4 | 18.40: 0xAA632BC | 19.10: 0xC3BA164 | 21.30: 0x947F4FC | 22.40: 0x9BF411C | 23.50: 0x65C8CD4 | 30.00: 0x9C439DC
- **StaticFindObject** (مرسا "Illegal call to StaticFindObject" — موجود على UE4 فقط): 5.41: 0x3FE2D60 | 7.40: 0x4DB7CCC | 9.41: 0x5D9ABC8 | 13.40: 0xBCF5554 | 17.50: 0xDE7B67C | 18.40: 0xC7273CC
- **FFrame**: Node@+0x18, Code@+0x20 (مؤكد من الـ thunks)
- **Step helpers** (StepCompiledIn family): 21.30: 0x760ECC8 (StepExplicitProperty), 0x75E0414, 0x75CC0F0, 0x760ED48

### 3) سلاسل البناء (كشف الإصدار على أندرويد — أسهل من جدول CL!)
النص `++Fortnite+Release-X.YY` موجود مباشرة في .rodata كـ UTF-16 — اقرأه من الذاكرة ولا تحتاج جدول CL إطلاقاً.

### 4) مطابقة Windows (21.30 exe) — للتوثيق العابر للمعماريات
| الدالة | Windows x64 | أندرويد ARM64 |
|---|---|---|
| EquipVehicleWeapon | 0x1471CD83C (جسم حقيقي) | 0x2FB593C (ret ICF — مختلف!) |
| UnEquipVehicleWeapon | 0x1471DB2A0 | 0x6A1C864 ✓ |
| GetEngineVersion | 0x14855E6F8 | — |
| FFrame::Step | 0x140E78448 | عائلة 0x760ECC8 |

---

## 5) أخطاء يجب إصلاحها في كودك

1. **[حرج] EquipVehicleWeapon على UE5 (>= 19.10)**: غيّر إلى `Hooking::ExecHook(UFunction)` — الـ impl مطوي ICF
2. **[حرج] OnPawnEnterSeat 30.00**: استبدل النمط بالنمط المصحح أعلاه (الهدف 0xA160AAC)
3. **[متوسط] 13.40 OnPawnEnterSeat**: نمطك لا يغطيها و PostLoadHook يستدعيها — أضف نمطاً أو استخدم ExecHook على NativeOnPawnEnterSeat (توجد كـ native من 19.10 فقط، فعلى 13.40 استخدم التتبع البنيوي)
4. **[منخفض] UnEquip على 5.41-9.41**: لا فروع — مثل EquipVW (والمسار معطل عندك أصلاً بالتعليق)
