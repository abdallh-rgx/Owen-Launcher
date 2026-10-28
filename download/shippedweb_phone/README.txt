==========================================================================
  OWEN PARTYHUB — Voltronite Edition (v3)
  النسخة الجديدة من shippedweb — تعمل مباشرة على الهاتف
==========================================================================

محتويات الحزمة
--------------
  shippedweb/
    server.py                 الخادم المحلي (بايثون فقط، بدون أي مكتبات إضافية)
    offline/entry/            نسخة PartyHub المرقّعة بالكامل
      owen-bridge.js          ★ الجسر: الروابط + صفحة الدخول الذهبية ★
    README.txt                هذا الملف

ما الجديد في هذه النسخة
------------------------
  1. تم حذف الشريط الأحمر (صندوق الـ debug) الذي كان يظهر في أعلى الشاشة
     نهائياً — الشاشة نظيفة تماماً.

  2. صفحة تسجيل دخول جديدة بنفس تصميم الـ Launcher الذهبي
     (خلفية فحمية + زر ذهبي متدرج + بطاقة اسم اللاعب).
     اكتب اسمك واضغط LOGIN — الاسم يُرسل إلى Voltronite ويصبح حسابك
     (accountId = hex(الاسم) بنفس طريقة Voltronite تماماً).

  3. الروابط في owen-bridge.js أصبحت متوافقة مع Voltronite:
     كل استعلامات graphql تُحوَّل تلقائياً إلى عمليات Voltronite
     (GetMyAccount / GetStatusPerService / GetMySetting / ...) وتُرسل
     مع توكن حقيقي إلى Voltronite عبر الخادم المحلي.

التشغيل على الهاتف (Termux)
---------------------------
  1. ثبّت Termux من F-Droid، ثم داخل Termux:
         pkg install python

  2. فك ضغط الحزمة في أي مكان، مثلاً:
         cd ~
         (انسخ مجلد shippedweb إلى الهاتف ثم)
         cd shippedweb

  3. (مهم) إذا كان Voltronite يعمل على الكمبيوتر وليس على الهاتف:
     افتح الملف offline/entry/owen-bridge.js وغيّر السطر:
         VOLTRONITE_URL: "http://127.0.0.1:3551"
     إلى عنوان الكمبيوتر على نفس شبكة الواي فاي، مثلاً:
         VOLTRONITE_URL: "http://192.168.1.20:3551"
     (الهاتف والكمبيوتر يجب أن يكونا على نفس الشبكة)

  4. شغّل الخادم:
         python server.py

  5. افتح كروم على الهاتف وادخل إلى:
         http://127.0.0.1:8081

  6. ستظهر صفحة الدخول الذهبية (OWEN LAUNCHER):
     اكتب اسمك → اضغط LOGIN → تدخل واجهة PartyHub كاملة.

ملاحظات
--------
  - حتى لو كان Voltronite غير متصل، التطبيق يعمل ويعرض الواجهة
    (ببيانات محلية) — وعند اتصاله تصل الطلبات فوراً.
  - سجل الخادم يطبع كل طلب يصل إلى Voltronite:
        [RELAY] GetMyAccount  user=اسمك  -> 200
  - لتغيير المنفذ:  PORT=9000 python server.py
  - الخروج من الحساب داخل التطبيق يعيد صفحة الدخول الذهبية.

تشغيل على الكمبيوتر (اختياري)
------------------------------
  نفس الخطوات: فك الضغط ثم  python server.py  ثم افتح
  http://127.0.0.1:8081  (يتطلب بايثون 3.7 أو أحدث)

==========================================================================
  ENGLISH QUICK START
==========================================================================
  Requirements: Python 3.7+ (Termux: pkg install python)

  1. Extract this archive, cd into the shippedweb folder.
  2. If Voltronite runs on another machine, edit
     offline/entry/owen-bridge.js  ->  OWEN_CONFIG.VOLTRONITE_URL
     and point it at that machine (e.g. http://192.168.1.20:3551).
  3. python server.py
  4. Open http://127.0.0.1:8081 in the browser.
  5. Golden login page -> type your name -> LOGIN -> PartyHub.

  The relay log prints every request that reaches Voltronite:
     [RELAY] GetMyAccount  user=<your name>  -> 200
==========================================================================
