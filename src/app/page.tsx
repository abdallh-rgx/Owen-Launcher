"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  Search,
  Download,
  Github,
  FunctionSquare,
  Database,
  ChevronRight,
  ChevronLeft,
  Copy,
  Check,
  Loader2,
  Package,
  Crosshair,
  ExternalLink,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Separator } from "@/components/ui/separator";

const VERSIONS = [
  "5.41",
  "7.40",
  "9.41",
  "13.40",
  "17.50",
  "18.40",
  "19.10",
  "21.30",
  "22.40",
  "23.50",
  "30.00",
] as const;
type Ver = (typeof VERSIONS)[number];

interface FuncEntry {
  thunk: string;
  orig?: string;
}
interface SearchHit {
  name: string;
  versions: Partial<Record<Ver, FuncEntry>>;
}
interface Stats {
  perVersion: Record<string, number>;
  unique: number;
  inAll: number;
}
interface ErbiumEntry {
  [ver: string]: {
    addr?: string | null;
    len?: number | null;
    pattern?: string;
    strategy?: string;
    global?: string;
    vtable_index?: number;
    verified?: boolean;
  };
}
interface P1Info {
  win?: string;
  arm?: string;
  how?: string;
  size_ratio?: number;
  strings_match?: boolean | null;
  natives_match?: boolean | null;
  strings_common?: string[];
  votes?: number;
}

function shortAddr(a?: string | null): string {
  if (!a) return "—";
  return a.replace(/^0x/, "").toUpperCase();
}

function copyText(text: string) {
  navigator.clipboard?.writeText(text);
}

export default function Home() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [tab, setTab] = useState("search");

  useEffect(() => {
    fetch("/api/db/stats")
      .then((r) => r.json())
      .then(setStats)
      .catch(() => {});
  }, []);

  return (
    <div dir="rtl" className="min-h-screen flex flex-col bg-background">
      <header className="sticky top-0 z-40 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/80">
        <div className="mx-auto max-w-[1400px] px-4 py-3 flex items-center gap-3 flex-wrap">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
              <Database className="h-5 w-5" />
            </div>
            <div>
              <h1 className="text-lg font-bold leading-tight">
                قاعدة دوال فورتنايت أندرويد
              </h1>
              <p className="text-xs text-muted-foreground">
                Fortnite Android ARM64 Native Functions DB — 5.41 → 30.00
              </p>
            </div>
          </div>
          <div className="flex-1" />
          <div className="flex items-center gap-2 flex-wrap">
            {stats && (
              <>
                <Badge variant="secondary" className="gap-1">
                  <FunctionSquare className="h-3 w-3" />
                  {stats.unique.toLocaleString("en")} دالة
                </Badge>
                <Badge variant="secondary">11 نسخة</Badge>
                <Badge variant="outline" className="gap-1">
                  <Crosshair className="h-3 w-3" />
                  دوال Erbium بأنماط ARM64
                </Badge>
              </>
            )}
            <Button asChild size="sm" variant="outline" className="gap-1">
              <a
                href="https://github.com/abdallh-rgx/Erbium-Android"
                target="_blank"
                rel="noreferrer"
              >
                <Github className="h-4 w-4" />
                GitHub
                <ExternalLink className="h-3 w-3" />
              </a>
            </Button>
          </div>
        </div>
      </header>

      <main className="flex-1 mx-auto w-full max-w-[1400px] px-4 py-6">
        <Tabs value={tab} onValueChange={setTab} className="w-full">
          <TabsList className="mb-4 flex-wrap h-auto">
            <TabsTrigger value="search" className="gap-1.5">
              <Search className="h-4 w-4" /> بحث في الدوال
            </TabsTrigger>
            <TabsTrigger value="versions" className="gap-1.5">
              <Package className="h-4 w-4" /> تصفح النسخ
            </TabsTrigger>
            <TabsTrigger value="erbium" className="gap-1.5">
              <Crosshair className="h-4 w-4" /> دوال Erbium
            </TabsTrigger>
            <TabsTrigger value="download" className="gap-1.5">
              <Download className="h-4 w-4" /> تحميل الملفات
            </TabsTrigger>
          </TabsList>

          <TabsContent value="search">
            <SearchTab stats={stats} />
          </TabsContent>
          <TabsContent value="versions">
            <VersionsTab stats={stats} />
          </TabsContent>
          <TabsContent value="erbium">
            <ErbiumTab />
          </TabsContent>
          <TabsContent value="download">
            <DownloadTab />
          </TabsContent>
        </Tabs>
      </main>

      <footer className="mt-auto border-t bg-muted/30">
        <div className="mx-auto max-w-[1400px] px-4 py-4 text-xs text-muted-foreground flex flex-wrap items-center gap-x-4 gap-y-2 justify-between">
          <span>
            استخراج ثابت من libUnreal.so / libUE4.so — العناوين RVA (base + RVA)
          </span>
          <span>
            تحقق 100% (20,965/20,965) · للبحث والتعليم · Erbium Android Port
          </span>
        </div>
      </footer>
    </div>
  );
}

/* ================= Search Tab ================= */

function SearchTab({ stats: _stats }: { stats: Stats | null }) {
  const [q, setQ] = useState("");
  const [mode, setMode] = useState<"contains" | "prefix" | "exact">("contains");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [total, setTotal] = useState(0);
  const [ms, setMs] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<SearchHit | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!q.trim()) return;
    debounceRef.current = setTimeout(() => {
      setLoading(true);
      fetch(`/api/db/search?q=${encodeURIComponent(q)}&mode=${mode}&limit=80`)
        .then((r) => r.json())
        .then((d) => {
          setHits(d.hits ?? []);
          setTotal(d.total ?? 0);
          setMs(d.ms ?? null);
        })
        .finally(() => setLoading(false));
    }, 300);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [q, mode]);

  const hasQuery = q.trim().length > 0;

  const examples = useMemo(
    () => [
      "ServerHandlePickup",
      "ServerMove",
      "OnRep_",
      "EnterAircraft",
      "Zipline",
      "PickTeam",
      "CheatManager",
      "GiveAbility",
    ],
    []
  );

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">
            ابحث عن أي دالة — بالاسم، عبر كل النسخ (49,859 دالة)
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex gap-2 flex-wrap">
            <div className="relative flex-1 min-w-[240px]">
              <Search className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                dir="ltr"
                value={q}
                onChange={(e) => setQ(e.target.value)}
                placeholder="ServerHandlePickup, OnRep_, Zipline..."
                className="pr-9 text-left font-mono"
                autoFocus
              />
              {loading && (
                <Loader2 className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 animate-spin text-muted-foreground" />
              )}
            </div>
            <Select
              value={mode}
              onValueChange={(v) => setMode(v as typeof mode)}
            >
              <SelectTrigger className="w-[140px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="contains">يحتوي على</SelectItem>
                <SelectItem value="prefix">يبدأ بـ</SelectItem>
                <SelectItem value="exact">مطابقة تامة</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <span>أمثلة سريعة:</span>
            {examples.map((ex) => (
              <button
                key={ex}
                dir="ltr"
                onClick={() => setQ(ex)}
                className="font-mono rounded-md border px-2 py-0.5 hover:bg-accent transition-colors"
              >
                {ex}
              </button>
            ))}
          </div>
          {(total > 0 || ms !== null) && hasQuery && (
            <p className="text-xs text-muted-foreground">
              {total.toLocaleString("en")} نتيجة
              {ms !== null && total > 0 ? ` · ${ms}ms` : ""}
              {total > 80 ? " · تُعرض أول 80 — ضيّق البحث للمزيد" : ""}
            </p>
          )}
        </CardContent>
      </Card>

      {hasQuery && hits.length > 0 && (
        <Card>
          <CardContent className="p-0">
            <div className="overflow-x-auto max-h-[65vh] overflow-y-auto">
              <Table>
                <TableHeader className="sticky top-0 z-10 bg-background">
                  <TableRow>
                    <TableHead className="sticky right-0 z-20 bg-background text-right min-w-[220px]">
                      اسم الدالة
                    </TableHead>
                    {VERSIONS.map((v) => (
                      <TableHead key={v} className="text-center font-mono text-xs">
                        {v}
                      </TableHead>
                    ))}
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {hits.map((h) => (
                    <TableRow
                      key={h.name}
                      className="cursor-pointer"
                      onClick={() => setSelected(h)}
                    >
                      <TableCell className="sticky right-0 z-10 bg-background font-mono text-xs text-left whitespace-nowrap border-l">
                        <span className="text-primary">{h.name}</span>
                      </TableCell>
                      {VERSIONS.map((v) => {
                        const e = h.versions[v];
                        return (
                          <TableCell
                            key={v}
                            className={`text-center font-mono text-[10px] whitespace-nowrap ${
                              e
                                ? "cursor-pointer hover:bg-accent"
                                : "text-muted-foreground/40"
                            }`}
                            onClick={(ev) => {
                              if (e) {
                                ev.stopPropagation();
                                copyText(e.thunk);
                              }
                            }}
                            title={e ? `thunk: ${e.thunk}${e.orig ? ` | orig: ${e.orig}` : ""} — اضغط للنسخ` : "غير موجودة في هذه النسخة"}
                          >
                            {e ? shortAddr(e.thunk) : "·"}
                          </TableCell>
                        );
                      })}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
            <div className="px-4 py-2 text-[10px] text-muted-foreground border-t">
              القيم = عنوان الـ thunk (RVA) · اضغط على خانة لنسخ العنوان · اضغط
              على الصف للتفاصيل الكاملة
            </div>
          </CardContent>
        </Card>
      )}

      <FunctionDetailDialog hit={selected} onClose={() => setSelected(null)} />
    </div>
  );
}

function FunctionDetailDialog({
  hit,
  onClose,
}: {
  hit: SearchHit | null;
  onClose: () => void;
}) {
  const [copied, setCopied] = useState<string | null>(null);
  if (!hit) return null;
  const vers = Object.keys(hit.versions).sort(
    (a, b) => VERSIONS.indexOf(a as Ver) - VERSIONS.indexOf(b as Ver)
  );
  return (
    <Dialog open={!!hit} onOpenChange={(o) => !o && onClose()}>
      <DialogContent dir="rtl" className="max-w-2xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle dir="ltr" className="text-left font-mono text-primary">
            {hit.name}
          </DialogTitle>
          <DialogDescription>
            متوفرة في {vers.length} من 11 نسخة — العناوين RVA (base + RVA)
          </DialogDescription>
        </DialogHeader>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="text-right">النسخة</TableHead>
              <TableHead className="text-right">Thunk (ExecHook)</TableHead>
              <TableHead className="text-right">Orig (Impl)</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {vers.map((v) => {
              const e = hit.versions[v as Ver]!;
              const key = `${v}:t`;
              return (
                <TableRow key={v}>
                  <TableCell className="font-mono font-bold">{v}</TableCell>
                  <TableCell className="font-mono text-xs" dir="ltr">
                    <button
                      className="hover:text-primary"
                      onClick={() => {
                        copyText(e.thunk);
                        setCopied(key);
                        setTimeout(() => setCopied(null), 1200);
                      }}
                    >
                      {e.thunk} {copied === key ? <Check className="inline h-3 w-3 text-green-600" /> : <Copy className="inline h-3 w-3 opacity-40" />}
                    </button>
                  </TableCell>
                  <TableCell className="font-mono text-xs" dir="ltr">
                    {e.orig ? (
                      <button
                        className="hover:text-primary"
                        onClick={() => {
                          copyText(e.orig!);
                          setCopied(`${v}:o`);
                          setTimeout(() => setCopied(null), 1200);
                        }}
                      >
                        {e.orig} {copied === `${v}:o` ? <Check className="inline h-3 w-3 text-green-600" /> : <Copy className="inline h-3 w-3 opacity-40" />}
                      </button>
                    ) : (
                      <span className="text-muted-foreground">مطوي/نفس الـ thunk</span>
                    )}
                  </TableCell>
                  <TableCell />
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </DialogContent>
    </Dialog>
  );
}

/* ================= Versions Tab ================= */

function VersionsTab({ stats }: { stats: Stats | null }) {
  const [ver, setVer] = useState<string>("21.30");
  const [page, setPage] = useState(1);
  const [q, setQ] = useState("");
  const [data, setData] = useState<{
    functions: { name: string; thunk: string; orig: string | null }[];
    total: number;
    totalPages: number;
  } | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const t = setTimeout(() => {
      setLoading(true);
      fetch(
        `/api/db/version?ver=${ver}&page=${page}&pageSize=100&q=${encodeURIComponent(q)}`
      )
        .then((r) => r.json())
        .then((d) => setData(d))
        .finally(() => setLoading(false));
    }, 250);
    return () => clearTimeout(t);
  }, [ver, page, q]);

  const changeVer = (v: string) => {
    setVer(v);
    setPage(1);
  };
  const changeQ = (v: string) => {
    setQ(v);
    setPage(1);
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 lg:grid-cols-11 gap-2">
        {VERSIONS.map((v) => (
          <button
            key={v}
            onClick={() => changeVer(v)}
            className={`rounded-lg border p-2 text-center transition-colors ${
              ver === v
                ? "border-primary bg-primary/10"
                : "hover:bg-accent"
            }`}
          >
            <div className="font-mono font-bold text-sm">{v}</div>
            <div className="text-[10px] text-muted-foreground">
              {stats?.perVersion?.[v]
                ? stats.perVersion[v].toLocaleString("en")
                : "…"}
            </div>
          </button>
        ))}
      </div>

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center gap-2 flex-wrap">
            <h3 className="font-bold">نسخة {ver}</h3>
            {data && (
              <Badge variant="secondary">
                {data.total.toLocaleString("en")} دالة
              </Badge>
            )}
            <div className="flex-1" />
            <div className="relative w-64">
              <Search className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                dir="ltr"
                className="pr-9 font-mono text-left"
                placeholder="فلترة بالاسم..."
                value={q}
                onChange={(e) => changeQ(e.target.value)}
              />
            </div>
          </div>

          <div className="border rounded-lg overflow-auto max-h-[60vh]">
            <Table>
              <TableHeader className="sticky top-0 bg-background">
                <TableRow>
                  <TableHead className="text-right">#</TableHead>
                  <TableHead className="text-right">الاسم</TableHead>
                  <TableHead className="text-right">Thunk RVA</TableHead>
                  <TableHead className="text-right">Orig RVA</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {loading && !data && (
                  <TableRow>
                    <TableCell colSpan={4} className="text-center py-8">
                      <Loader2 className="inline h-5 w-5 animate-spin" />
                    </TableCell>
                  </TableRow>
                )}
                {data?.functions.map((f, i) => (
                  <TableRow
                    key={f.name}
                    className="cursor-pointer"
                    onClick={() => copyText(f.thunk)}
                    title="اضغط لنسخ الـ thunk"
                  >
                    <TableCell className="text-xs text-muted-foreground">
                      {(page - 1) * 100 + i + 1}
                    </TableCell>
                    <TableCell
                      dir="ltr"
                      className="font-mono text-xs text-left text-primary whitespace-nowrap"
                    >
                      {f.name}
                    </TableCell>
                    <TableCell dir="ltr" className="font-mono text-xs">
                      {f.thunk}
                    </TableCell>
                    <TableCell dir="ltr" className="font-mono text-xs text-muted-foreground">
                      {f.orig ?? "—"}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          {data && data.totalPages > 1 && (
            <div className="flex items-center justify-center gap-3">
              <Button
                size="sm"
                variant="outline"
                disabled={page <= 1}
                onClick={() => setPage((p) => p - 1)}
              >
                <ChevronRight className="h-4 w-4" /> السابق
              </Button>
              <span className="text-xs text-muted-foreground font-mono">
                {page} / {data.totalPages}
              </span>
              <Button
                size="sm"
                variant="outline"
                disabled={page >= data.totalPages}
                onClick={() => setPage((p) => p + 1)}
              >
                التالي <ChevronLeft className="h-4 w-4" />
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

/* ================= Erbium Tab ================= */

function ErbiumTab() {
  const [data, setData] = useState<Record<string, ErbiumEntry> | null>(null);
  const [p1, setP1] = useState<Record<string, P1Info> | null>(null);

  useEffect(() => {
    fetch("/api/db/download?group=erbium&file=erbium-summary.json&inline=1")
      .then((r) => r.json())
      .then(setData)
      .catch(() => {});
    fetch("/api/db/download?group=erbium&file=verify_p1_v3.json&inline=1")
      .then((r) => r.json())
      .then(setP1)
      .catch(() => {});
  }, []);

  const funcs = useMemo(() => {
    if (!data) return [];
    return Object.keys(data).sort();
  }, [data]);

  const summary = useMemo(() => {
    if (!data) return null;
    let cells = 0, patterns = 0, globals = 0, vtidx = 0, full = 0;
    let inline = 0, inlineCells = 0;
    for (const f of Object.keys(data)) {
      let ok = 0;
      for (const v of Object.keys(data[f])) {
        const e = data[f][v];
        if (e?.pattern && e?.addr) { cells++; patterns++; ok++; }
        else if (e?.global) { cells++; globals++; ok++; }
        else if (e?.vtable_index != null) { cells++; vtidx++; ok++; }
      }
      if (ok === 11) full++;
      if (f.startsWith("Inline_")) { inline++; inlineCells += ok; }
    }
    return { cells, patterns, globals, vtidx, full, total: Object.keys(data).length, inline, inlineCells };
  }, [data]);

  const [filter, setFilter] = useState("");
  const [onlyInline, setOnlyInline] = useState(false);
  const filtered = funcs.filter(
    (f) =>
      (!filter || f.toLowerCase().includes(filter.toLowerCase())) &&
      (!onlyInline || f.startsWith("Inline_"))
  );

  return (
    <div className="space-y-4">
      <Card className="border-emerald-500/30 bg-emerald-500/5">
        <CardContent className="p-4">
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <Badge variant="default" className="bg-emerald-600">v6 مُتحقق على الثنائيات الحقيقية</Badge>
            <span className="text-muted-foreground">
              استُعيدت <b>11 ثنائي أندرويد حقيقي</b> (5.2.0→32.00 بينها 10.40-CL-8936006 نسخة Erbium المرجعية
              و21.30 بنفس Build-ID الجهاز) وأعيد التحقق من كل خلية: <b>أُزيلت 6 مطابقات خاطئة</b> من v5
              (CHILF@21.30 + CreateNetDriverWorldContext×5 — كانت عوامل نسخ/عدادات)، وأُضيفت سلاسل v6
              المُتحققة: <b>next-FDE عبر .eh_frame</b> (تطابق حرفي 6/7 GT) · نمط <b>SetPickupItems@13.40</b>
              (فريد 0xc674418) · نمط <b>SetChannelActor</b> لعائلة 5.x (فريد على 5.2.0) · سلسلة
              <b>LogV2 descriptor</b> لعائلة UE5.4/5.5 (مثبتة على 32.00 → 0x4f15ee0) · ماسح
              <b>GNames الغاشم</b> للنسخ القديمة · تنفيذ <b>CHILF أصلي مضمون</b> — اختبارات وظيفية
              <b>16/16</b> — التغطية <b>979/979 (100%)</b> مع توثيق أمين لكل خلية.
            </span>
          </div>
          <div className="mt-2 flex flex-wrap gap-2 text-xs">
            {["5.41","7.40","9.41","13.40","17.50","18.40","19.10","21.30","22.40","23.50","30.00"].map((v) => (
              <a
                key={v}
                href={`/api/db/download?group=erbium&file=offsets/ERBIUM-OFFSETS-${v}.txt&inline=1`}
                className="rounded-md border px-2 py-1 hover:bg-accent font-mono"
                target="_blank"
                rel="noreferrer"
              >
                أوفستات {v}
              </a>
            ))}
          </div>
        </CardContent>
      </Card>
      {summary && (
        <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
          <Card className="border-emerald-500/40 bg-emerald-500/5">
            <CardContent className="p-4">
              <div className="text-2xl font-bold text-emerald-600">979/979</div>
              <div className="text-xs text-muted-foreground">تغطية v6 (نمط + سلاسل وقت-تشغيل مُتحققة)</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="text-2xl font-bold text-primary">{summary.patterns}</div>
              <div className="text-xs text-muted-foreground">نمط ARM64 مُتحقق (مطابقة واحدة بالضبط)</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="text-2xl font-bold text-primary">{summary.cells}/979</div>
              <div className="text-xs text-muted-foreground">حل ثابت (أنماط/globals/vtable) — قبل RT::</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="text-2xl font-bold text-primary">{summary.full}</div>
              <div className="text-xs text-muted-foreground">دالة مغطاة 11/11 نسخة</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="text-2xl font-bold text-primary">{summary.inline}</div>
              <div className="text-xs text-muted-foreground">دالة إضافية من خارج Finders.cpp ({summary.inlineCells} خلية)</div>
            </CardContent>
          </Card>
        </div>
      )}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base flex items-center gap-2">
            <Crosshair className="h-4 w-4" />
            دوال Erbium — تحقق ثلاثي: P0 (مطابقة فريدة) + P1 (مقارنة ويندوز) + P2 (native-identity)
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm text-muted-foreground">
          <p>
            كل دالة لها نمط بايتات ARM64 مطابق <b>مرة واحدة بالضبط</b> في ثنائية نسختها (تحقق آلي P0)،
            ومقارنة سماتها (نصوص/natives/حجم) ضد <code dir="ltr">FortniteClient-Win64-Shipping.exe</code> (P1)،
            ومطابقة عناوين natives من الـ dumps (P2). الأنماط الكاملة في{" "}
            <code dir="ltr">Finders-Android-FULL-v3.cpp</code>.
          </p>
          <p className="text-xs">
            القيم = العنوان (RVA) · G = global عبر GOT · VT = فهرس vtable محسوب · ✕ = غير محلول ·
            الدوال المسبوقة <code dir="ltr">Inline_</code> من ملفات Erbium الأخرى (خارج Finders.cpp)
          </p>
          <div className="flex items-center gap-3 flex-wrap">
            <div className="relative w-64">
              <Search className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                dir="ltr"
                className="pr-9 font-mono text-left"
                placeholder="FindServerMove... / Inline_"
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
              />
            </div>
            <label className="flex items-center gap-2 text-xs cursor-pointer">
              <input
                type="checkbox"
                checked={onlyInline}
                onChange={(e) => setOnlyInline(e.target.checked)}
                className="accent-primary"
              />
              دوال خارج Finders.cpp فقط
            </label>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-0">
          <div className="overflow-x-auto max-h-[65vh] overflow-y-auto">
            <Table>
              <TableHeader className="sticky top-0 z-10 bg-background">
                <TableRow>
                  <TableHead className="sticky right-0 z-20 bg-background text-right min-w-[260px]">
                    الدالة
                  </TableHead>
                  {VERSIONS.map((v) => (
                    <TableHead key={v} className="text-center font-mono text-xs">
                      {v}
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {!data && (
                  <TableRow>
                    <TableCell colSpan={12} className="text-center py-8">
                      <Loader2 className="inline h-5 w-5 animate-spin" />
                    </TableCell>
                  </TableRow>
                )}
                {filtered.map((f) => {
                  const p = p1?.[f];
                  const p1Badge = p
                    ? p.votes != null && p.votes >= 2
                      ? "P1✓"
                      : typeof p.size_ratio === "number" && p.size_ratio <= 4
                        ? "P1≈"
                        : "P1?"
                    : "";
                  return (
                  <TableRow key={f}>
                    <TableCell
                      dir="ltr"
                      className="sticky right-0 z-10 bg-background font-mono text-xs text-left whitespace-nowrap border-l text-primary"
                      title={p ? `P1: win=${p.win} ratio=${p.size_ratio} strs=${p.strings_match} · ${p.strings_common?.join(" | ") ?? ""}` : ""}
                    >
                      <span className={f.startsWith("Inline_") ? "text-amber-600 dark:text-amber-400" : ""}>{f}</span>
                      {p1Badge && (
                        <span
                          className={`ms-2 inline-block rounded px-1 text-[9px] font-bold ${
                            p1Badge === "P1✓"
                              ? "bg-green-500/15 text-green-600"
                              : p1Badge === "P1≈"
                                ? "bg-emerald-500/10 text-emerald-600"
                                : "bg-amber-500/15 text-amber-600"
                          }`}
                        >
                          {p1Badge}
                        </span>
                      )}
                    </TableCell>
                    {VERSIONS.map((v) => {
                      const e = data?.[f]?.[v];
                      const label = e?.addr
                        ? shortAddr(e.addr)
                        : e?.global
                          ? `G:${shortAddr(e.global)}`
                          : e?.vtable_index != null
                            ? `VT:${e.vtable_index}`
                            : e
                              ? "✓"
                              : "✕";
                      const tip = e
                        ? `${e.addr ?? e.global ?? e.vtable_index ?? ""} · ${e.strategy ?? ""}${e.verified === false ? " · ⚠️ غير مُتحقق" : ""}`
                        : "";
                      return (
                        <TableCell
                          key={v}
                          className={`text-center font-mono text-[10px] whitespace-nowrap ${
                            e ? "cursor-pointer hover:bg-accent" : "text-muted-foreground/40"
                          }`}
                          onClick={() => e && copyText(e.addr ?? e.global ?? String(e.vtable_index ?? ""))}
                          title={tip || undefined}
                        >
                          {label}
                        </TableCell>
                      );
                    })}
                  </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

/* ================= Download Tab ================= */

function DownloadTab() {
  const files = useMemo(
    () => [
      { group: "db", name: "by-name.json", desc: "الفهرس العكسي الكامل — كل دالة × كل النسخ (12MB)" },
      ...VERSIONS.map((v) => ({
        group: "db",
        name: `natives-v${v}.json`,
        desc: `كل دوال نسخة ${v}`,
      })),
      { group: "db", name: "version-diffs.json", desc: "الفروق بين النسخ (مضاف/محذوف)" },
      { group: "db", name: "erbium-native-map.json", desc: "ربط دوال Erbium بالـ natives" },
      { group: "erbium", name: "Finders-Android-FULL-v6.cpp", desc: "⭐⭐ v6: كل الدوال 89 + محرك RT:: + سلاسل مُتحققة على 11 ثنائي حقيقي (اختبارات 16/16)" },
      { group: "erbium", name: "VERIFICATION-REPORT-v6.md", desc: "⭐⭐ تقرير التحقق v6: الثنائيات الحقيقية + كل سلسلة + الخلايا الخاطئة المُزالة" },
      { group: "erbium", name: "COVERAGE-MATRIX-v6.md", desc: "⭐⭐ مصفوفة v6: 979/979 (100%) مع توثيق أمين" },
      { group: "erbium", name: "Finders-Android-FULL-v5.cpp", desc: "v5: محرك RT:: للإيجاد الغاشم وقت-التشغيل (مُختبَر 19/19)" },
      { group: "erbium", name: "RUNTIME-SUPPORT-v5.md", desc: "توثيق محرك RT:: v5 + الأدلة + نتائج الاختبار الوظيفي" },
      { group: "erbium", name: "COVERAGE-MATRIX-v5.md", desc: "مصفوفة v5: 954/979 (97%)" },
      { group: "erbium", name: "Finders-Android-FULL-v4.cpp", desc: "v4: 717 نمط فريدة مُتحققة (بدون سلاسل وقت-التشغيل)" },
      { group: "erbium", name: "UNIVERSALITY-REPORT.md", desc: "تقرير الكونية — وضع no-patterns استعاد 728/1034" },
      { group: "erbium", name: "Finders-Android-FULL-v3.cpp", desc: "v3 — 100 دالة (83+17 خارج Finders.cpp) × 11 نسخة" },
      { group: "erbium", name: "VERIFICATION-REPORT-v3.md", desc: "تقرير التحقق v3 — المقارنة مع FortniteClient-Win64-Shipping.exe" },
      { group: "erbium", name: "COVERAGE-MATRIX-v3.md", desc: "مصفوفة تغطية v3 (94 دالة × 11 نسخة)" },
      { group: "erbium", name: "final_resolved_v4.json", desc: "كل الحلول النهائية v4 (عناوين/أنماط/فهارس) — JSON" },
      { group: "erbium", name: "verify_p1_v3.json", desc: "نتائج التحقق cross-arch ضد exe ويندوز" },
      { group: "erbium", name: "COVERAGE-MATRIX.md", desc: "مصفوفة التغطية (النسخة القديمة)" },
      { group: "erbium", name: "FINAL-REPORT.md", desc: "التقرير النهائي والمنهجية" },
    ],
    []
  );

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base flex items-center gap-2">
            <Download className="h-4 w-4" />
            تحميل قاعدة البيانات والملفات
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground mb-4">
            كل الملفات متوفرة أيضاً على{" "}
            <a
              href="https://github.com/abdallh-rgx/Erbium-Android"
              target="_blank"
              rel="noreferrer"
              className="text-primary underline inline-flex items-center gap-1"
            >
              GitHub: abdallh-rgx/Erbium-Android <ExternalLink className="h-3 w-3" />
            </a>{" "}
            (مستودع خاص على حسابك)
          </p>
          <Separator className="mb-4" />
          <div className="grid gap-2">
            {files.map((f) => (
              <div
                key={`${f.group}/${f.name}`}
                className="flex items-center gap-3 rounded-lg border p-3 hover:bg-accent/50 transition-colors"
              >
                <FileIcon name={f.name} />
                <div className="flex-1 min-w-0">
                  <div dir="ltr" className="font-mono text-sm text-left truncate">
                    {f.name}
                  </div>
                  <div className="text-xs text-muted-foreground truncate">
                    {f.desc}
                  </div>
                </div>
                <div className="flex gap-1">
                  <Button asChild size="sm" variant="outline">
                    <a
                      href={`/api/db/download?group=${f.group}&file=${f.name}`}
                      download={f.name}
                    >
                      <Download className="h-4 w-4 ml-1" /> تحميل
                    </a>
                  </Button>
                  <Button asChild size="sm" variant="ghost">
                    <a
                      href={`/api/db/download?group=${f.group}&file=${f.name}&inline=1`}
                      target="_blank"
                      rel="noreferrer"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function FileIcon({ name }: { name: string }) {
  const cls = "h-9 w-9 rounded-lg flex items-center justify-center text-white";
  if (name.endsWith(".json"))
    return (
      <div className={`${cls} bg-emerald-600`}>
        <Database className="h-4 w-4" />
      </div>
    );
  if (name.endsWith(".cpp") || name.endsWith(".h"))
    return (
      <div className={`${cls} bg-rose-600`}>
        <FunctionSquare className="h-4 w-4" />
      </div>
    );
  return (
    <div className={`${cls} bg-amber-600`}>
      <Download className="h-4 w-4" />
    </div>
  );
}
