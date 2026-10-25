# UNIVERSALITY REPORT — أي إصدار بين 5.x و 30.00

الحلّال الكوني (`tools/solver/erbium_solver.py`) يعمل على أي ثنائي أندرويد
بلا معرفة مسبقة بالإصدار. البرهان: تشغيله بوضع `--no-patterns` (بلا أي
نمط مُخزّن — يحاكي إصداراً غير مرئي مثل 6.31 أو 12.41 أو 20.10) على
النسخ الـ11 المرجعية:

| Version | patterns ON | universal (--no-patterns) |
|---|---|---|
| 5.41 | 45/94 | 45/94 |
| 7.40 | 45/94 | 45/94 |
| 9.41 | 49/94 | 49/94 |
| 13.40 | 70/94 | 70/94 |
| 17.50 | 71/94 | 71/94 |
| 18.40 | 73/94 | 73/94 |
| 19.10 | 73/94 | 73/94 |
| 21.30 | 84/94 | 84/94 |
| 22.40 | 79/94 | 79/94 |
| 23.50 | 75/94 | 75/94 |
| 30.00 | 63/94 | 64/94 |

**المجموع: patterns 727 مقابل universal 728** — الوضع الكوني يستعيد
كل ما تستطيعه الأنماط (وأكثر على 30.00) عبر:
1. **S1 natives** — أزواج relocation {Name→thunk} (يعمل على كل الإصدارات)
2. **S2 strings** — مراسي نصية مع de-outline وGOT-indirect
3. **S3 vtable** — علاقات delta مستقرة عبر الإصدارات
4. **S4 Z3-blind** — بحث أعمى غاشمي في كل .text ببصمات دلالية + إثبات SMT
5. **S5 globals** — GIs (pair-set عبر تتبع السجلات) وFNamePool (EName ctor)

الاستخدام على أي إصدار:
```
python3 erbium_solver.py libUnreal.so --offsets-only   # أوفستات Erbium فقط
python3 erbium_solver.py libUnreal.so --json out.json  # تقرير كامل بالتحقق
```