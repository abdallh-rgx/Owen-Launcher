"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import {
  Activity,
  BadgeCheck,
  BookOpen,
  Boxes,
  ChevronDown,
  Copy,
  Download,
  FileJson,
  Gauge,
  History,
  KeyRound,
  Layers,
  ListOrdered,
  Loader2,
  Menu,
  Network,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  SearchCode,
  Server,
  ShieldCheck,
  Trash2,
  Upload,
  Users,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
import { Separator } from "@/components/ui/separator";
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

interface OwenConfig {
  version: 1;
  server: {
    base_url: string;
    xmpp_host: string;
    xmpp_port: number;
    login_path: string;
    graphql_path: string;
  };
  redirect: {
    domains: string[];
    extra_rules: Array<{ match: string; replace_path: string; enabled?: boolean }>;
  };
  manifest: { mode: "auto" | "off" };
  webview: {
    login_url: string;
    force_cleartext: boolean;
    partyhub: "auto" | "off";
  };
  detect: {
    strategy: "auto" | "symbols-only" | "signature-only";
    min_score: number;
  };
  hooks: { curl: boolean; art_webview: boolean };
}

interface StatusPayload {
  redirecter: { configured: boolean; updatedAt: string | null };
  rules: number;
  servers: number;
  seasons: number;
  keys: number;
  scans: Array<{
    id: string;
    fileName: string;
    bestVa: string | null;
    score: number;
    variadic: boolean;
    createdAt: string;
  }>;
}

interface DetectCandidate {
  score: number;
  variadic: boolean;
  funcStart: string | null;
  cbzX0: string | null;
  bl: string | null;
  blTarget: string | null;
  movW02b: string | null;
  gap: string;
  va: string | null;
}

interface DetectResponse {
  fileName: string;
  sizeBytes: number;
  elapsedMs: number;
  candidates: DetectCandidate[];
  best: DetectCandidate | null;
}

interface SeasonSummary {
  id: string;
  seasonNumber: number;
  displayName: string;
  paidCount: number;
  freeCount: number;
  hasBundle: boolean;
  updatedAt: string;
}

interface SeasonDetail {
  id: string;
  seasonNumber: number;
  displayName: string;
  updatedAt: string;
  data: {
    battleBundleOfferId?: string;
    battlePassOfferId?: string;
    tierOfferId?: string;
    paidRewards?: Array<Record<string, number>>;
    freeRewards?: Array<Record<string, number>>;
  };
}

interface ServerProfile {
  id: string;
  name: string;
  baseUrl: string;
  backend: string;
  features: string[];
}

interface KeyEntry {
  id: string;
  key: string;
  role: string;
  note: string;
  revoked: boolean;
  lastUsedAt: string | null;
  createdAt: string;
}

type TabId =
  | "dashboard"
  | "redirecter"
  | "finder"
  | "partyhub"
  | "battlepass"
  | "servers"
  | "keys"
  | "docs";

const NAV_ITEMS: Array<{ id: TabId; label: string; icon: typeof Activity }> = [
  { id: "dashboard", label: "Dashboard", icon: Activity },
  { id: "redirecter", label: "Redirecter", icon: FileJson },
  { id: "finder", label: "Symbol Finder", icon: SearchCode },
  { id: "partyhub", label: "PartyHub and Login", icon: Users },
  { id: "battlepass", label: "Battle Pass", icon: Boxes },
  { id: "servers", label: "Servers", icon: Server },
  { id: "keys", label: "Keys", icon: KeyRound },
  { id: "docs", label: "Documentation", icon: BookOpen },
];

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

function SectionHeading({ title, description }: { title: string; description?: string }) {
  return (
    <div className="mb-5">
      <h2 className="text-lg font-semibold text-zinc-100">{title}</h2>
      {description ? <p className="mt-1 text-sm text-zinc-400">{description}</p> : null}
    </div>
  );
}

function CopyButton({ value, label }: { value: string; label: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <Button
      variant="ghost"
      size="icon"
      className="size-9 text-zinc-400 hover:text-zinc-100"
      aria-label={`Copy ${label}`}
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(value);
          setCopied(true);
          toast.success(`${label} copied to clipboard`);
          setTimeout(() => setCopied(false), 1500);
        } catch {
          toast.error("Clipboard copy failed");
        }
      }}
    >
      {copied ? <BadgeCheck className="size-4 text-emerald-400" /> : <Copy className="size-4" />}
    </Button>
  );
}

function formatDate(iso: string | null): string {
  if (!iso) return "Never";
  const d = new Date(iso);
  return d.toLocaleDateString("en-GB", { year: "numeric", month: "short", day: "numeric" });
}

// ---------------------------------------------------------------------------
// OWEN wordmark (inline SVG)
// ---------------------------------------------------------------------------

function OwenMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      role="img"
      aria-label="Owen Launcher logo"
    >
      <rect x="1" y="1" width="30" height="30" rx="7" className="fill-zinc-900" stroke="currentColor" strokeWidth="2" />
      <circle cx="16" cy="16" r="9" stroke="currentColor" strokeWidth="3.5" />
      <rect x="14" y="3" width="4" height="6" rx="1" className="fill-emerald-400" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Dashboard section
// ---------------------------------------------------------------------------

const VERIFIED_COVERAGE: Array<{ lib: string; version: string; va: string; verification: string }> = [
  { lib: "libUE4.so", version: "10.10", va: "0x08EC981C", verification: "Exact match vs exported symbol" },
  { lib: "libUE4.so", version: "12.50", va: "0x0AE64C6C", verification: "Exact match vs exported symbol" },
  { lib: "libUnreal.so", version: "23.00", va: "0x0657E4A4", verification: "Verified by disassembly" },
  { lib: "libEOSSDK.so", version: "23.00", va: "0x0178AD00", verification: "Verified by disassembly" },
];

function DashboardSection({
  status,
  loading,
  onRefresh,
  onNavigate,
}: {
  status: StatusPayload | null;
  loading: boolean;
  onRefresh: () => void;
  onNavigate: (tab: TabId) => void;
}) {
  const stats: Array<{ label: string; value: string | undefined; hint: string; icon: typeof Gauge }> = [
    {
      label: "Redirecter config",
      value: status ? (status.redirecter.configured ? "Ready" : "Not set") : undefined,
      hint: status?.redirecter.configured ? "owen.json active" : "No base URL saved",
      icon: FileJson,
    },
    { label: "Active rules", value: status ? String(status.rules) : undefined, hint: "Extra redirect rules", icon: Layers },
    { label: "Servers", value: status ? String(status.servers) : undefined, hint: "Backend profiles", icon: Server },
    { label: "Seasons", value: status ? String(status.seasons) : undefined, hint: "Imported battle passes", icon: Boxes },
    { label: "Access keys", value: status ? String(status.keys) : undefined, hint: "Across all roles", icon: KeyRound },
  ];

  return (
    <div>
      <SectionHeading
        title="Dashboard"
        description="Operational overview of the Owen Launcher toolchain."
      />

      <div className="grid gap-4 lg:grid-cols-5 md:grid-cols-2 sm:grid-cols-1">
        <Card className="lg:col-span-2 border-zinc-800 bg-zinc-900">
          <CardContent className="flex items-start gap-4 p-6">
            <OwenMark className="size-12 text-emerald-400" />
            <div>
              <p className="text-xl font-semibold tracking-tight text-zinc-100">Owen Launcher</p>
              <p className="mt-1 text-sm leading-relaxed text-zinc-400">
                A no-smali redirect stack for Fortnite private servers. The Rust
                library <span className="font-mono text-emerald-400">libowen.so</span> is
                injected into the APK and auto-detects{" "}
                <span className="font-mono text-emerald-400">curl_easy_setopt</span> on any
                game version, from 10.31 through 33.00 and beyond.
              </p>
              <div className="mt-4 flex flex-wrap gap-2">
                <Badge className="border-emerald-400/30 bg-emerald-400/10 text-emerald-400 hover:bg-emerald-400/10">
                  no smali edits
                </Badge>
                <Badge className="border-zinc-700 bg-zinc-800 text-zinc-300 hover:bg-zinc-800">
                  Rust core
                </Badge>
                <Badge className="border-zinc-700 bg-zinc-800 text-zinc-300 hover:bg-zinc-800">
                  HTTP and HTTPS
                </Badge>
              </div>
            </div>
          </CardContent>
        </Card>

        {stats.map((s) => (
          <Card key={s.label} className="border-zinc-800 bg-zinc-900">
            <CardContent className="p-6">
              <div className="flex items-center justify-between">
                <p className="text-xs font-medium uppercase tracking-wide text-zinc-500">{s.label}</p>
                <s.icon className="size-4 text-emerald-400" aria-hidden="true" />
              </div>
              {loading ? (
                <div className="mt-3 h-8 w-20 animate-pulse rounded bg-zinc-800" />
              ) : (
                <p className="mt-3 text-2xl font-semibold text-zinc-100">{s.value ?? "0"}</p>
              )}
              <p className="mt-1 text-xs text-zinc-500">{s.hint}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="mt-6 grid gap-4 lg:grid-cols-2">
        <Card className="border-zinc-800 bg-zinc-900">
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center gap-2 text-base text-zinc-100">
              <ShieldCheck className="size-4 text-emerald-400" aria-hidden="true" />
              Verified coverage
            </CardTitle>
            <CardDescription className="text-zinc-500">
              Ground-truth validated <span className="font-mono">curl_easy_setopt</span> detections.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow className="border-zinc-800 hover:bg-transparent">
                  <TableHead className="text-zinc-400">Library</TableHead>
                  <TableHead className="text-zinc-400">Game version</TableHead>
                  <TableHead className="text-zinc-400">Detected VA</TableHead>
                  <TableHead className="text-zinc-400">Verification</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {VERIFIED_COVERAGE.map((row) => (
                  <TableRow key={`${row.lib}-${row.version}`} className="border-zinc-800">
                    <TableCell className="font-mono text-xs text-zinc-200">{row.lib}</TableCell>
                    <TableCell className="text-zinc-300">{row.version}</TableCell>
                    <TableCell className="font-mono text-xs text-emerald-400">
                      <span className="inline-flex items-center gap-1">
                        {row.va}
                        <CopyButton value={row.va} label={`VA ${row.va}`} />
                      </span>
                    </TableCell>
                    <TableCell className="text-zinc-400">{row.verification}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <div className="flex flex-col gap-4">
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center gap-2 text-base text-zinc-100">
                <ListOrdered className="size-4 text-emerald-400" aria-hidden="true" />
                Getting started
              </CardTitle>
              <CardDescription className="text-zinc-500">
                Four steps from config to a patched APK.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <ol className="space-y-3">
                {[
                  { title: "Configure the Redirecter", tab: "redirecter" as TabId, body: "Set the server base URL (HTTP or HTTPS, both first-class), XMPP host and redirect domains." },
                  { title: "Export owen.json to the device", tab: "redirecter" as TabId, body: "Download the generated file and push it to /sdcard/Android/data/<game>/files/ or /data/local/tmp/." },
                  { title: "Build libowen.so via CI", tab: "docs" as TabId, body: "Use the GitHub Actions workflow targeting aarch64-linux-android." },
                  { title: "Patch the APK with owen-apk", tab: "docs" as TabId, body: "owen-apk injects the library and the game loads it at startup." },
                ].map((step, i) => (
                  <li key={step.title} className="flex gap-3">
                    <span className="flex size-7 shrink-0 items-center justify-center rounded-full border border-emerald-400/30 bg-emerald-400/10 text-xs font-semibold text-emerald-400">
                      {i + 1}
                    </span>
                    <div>
                      <button
                        className="text-left text-sm font-medium text-zinc-200 hover:text-emerald-400"
                        onClick={() => onNavigate(step.tab)}
                      >
                        {step.title}
                      </button>
                      <p className="mt-0.5 text-xs leading-relaxed text-zinc-500">{step.body}</p>
                    </div>
                  </li>
                ))}
              </ol>
            </CardContent>
          </Card>

          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center gap-2 text-base text-zinc-100">
                <History className="size-4 text-emerald-400" aria-hidden="true" />
                Latest scans
              </CardTitle>
            </CardHeader>
            <CardContent>
              {!status || status.scans.length === 0 ? (
                <p className="text-sm text-zinc-500">
                  No scans yet. Upload a .so file in the Symbol Finder to see results here.
                </p>
              ) : (
                <ul className="max-h-48 space-y-2 overflow-y-auto pr-1">
                  {status.scans.map((scan) => (
                    <li
                      key={scan.id}
                      className="flex items-center justify-between gap-3 rounded-md border border-zinc-800 bg-zinc-950/60 px-3 py-2"
                    >
                      <div className="min-w-0">
                        <p className="truncate font-mono text-xs text-zinc-200">{scan.fileName}</p>
                        <p className="text-xs text-zinc-500">{formatDate(scan.createdAt)}</p>
                      </div>
                      <div className="flex shrink-0 items-center gap-2">
                        {scan.variadic ? (
                          <Badge className="border-emerald-400/30 bg-emerald-400/10 text-[10px] text-emerald-400 hover:bg-emerald-400/10">
                            va_start
                          </Badge>
                        ) : null}
                        <span className="font-mono text-xs text-emerald-400">{scan.bestVa ?? "none"}</span>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        </div>
      </div>

      <div className="mt-4">
        <Button
          variant="outline"
          onClick={onRefresh}
          disabled={loading}
          className="border-zinc-700 bg-zinc-900 text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100"
        >
          {loading ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
          Refresh status
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Redirecter section
// ---------------------------------------------------------------------------

const DEFAULT_CONFIG: OwenConfig = {
  version: 1,
  server: {
    base_url: "",
    xmpp_host: "",
    xmpp_port: 5222,
    login_path: "/login",
    graphql_path: "/partyhub/graphql",
  },
  redirect: {
    domains: [
      "game-social.epicgames.com",
      "ol.epicgames.com",
      "ol.epicgames.net",
      "on.epicgames.com",
      "ak.epicgames.com",
      "epicgames.dev",
    ],
    extra_rules: [],
  },
  manifest: { mode: "auto" },
  webview: { login_url: "", force_cleartext: true, partyhub: "auto" },
  detect: { strategy: "auto", min_score: 6 },
  hooks: { curl: true, art_webview: true },
};

function validateBaseUrl(v: string): string | null {
  const t = v.trim();
  if (!t) return "Server base URL is required.";
  if (!/^https?:\/\//i.test(t)) {
    return "URL must start with http:// or https:// (both schemes are accepted verbatim).";
  }
  try {
    const u = new URL(t);
    if (!u.hostname) return "URL must contain a host.";
  } catch {
    return "URL must contain a host (for example http://192.168.1.10:8000).";
  }
  return null;
}

function baseUrlScheme(v: string): "http" | "https" | null {
  const t = v.trim().toLowerCase();
  if (t.startsWith("http://")) return "http";
  if (t.startsWith("https://")) return "https";
  return null;
}

function RedirecterSection() {
  const [config, setConfig] = useState<OwenConfig>(DEFAULT_CONFIG);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [touched, setTouched] = useState(false);

  const set = useCallback(<K extends keyof OwenConfig>(key: K, value: OwenConfig[K]) => {
    setTouched(true);
    setConfig((c) => ({ ...c, [key]: value }));
  }, []);

  const setServer = useCallback(
    <K extends keyof OwenConfig["server"]>(key: K, value: OwenConfig["server"][K]) => {
      setTouched(true);
      setConfig((c) => ({ ...c, server: { ...c.server, [key]: value } }));
    },
    []
  );

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/redirecter");
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const json = await res.json();
        if (!cancelled && json.config) {
          setConfig({ ...DEFAULT_CONFIG, ...json.config });
          setTouched(false);
        }
      } catch {
        if (!cancelled) toast.error("Failed to load the Redirecter config");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const baseError = validateBaseUrl(config.server.base_url);
  const scheme = baseUrlScheme(config.server.base_url);
  const jsonPreview = useMemo(
    () =>
      JSON.stringify(config, (key, value) => (key === "enabled" ? undefined : value), 2),
    [config]
  );

  const downloadJson = () => {
    if (baseError) {
      toast.error("Fix the base URL error before downloading owen.json");
      return;
    }
    const blob = new Blob([jsonPreview], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "owen.json";
    a.click();
    URL.revokeObjectURL(url);
    toast.success("owen.json downloaded");
  };

  const save = async () => {
    if (baseError) {
      toast.error("Fix the base URL error before saving");
      return;
    }
    setSaving(true);
    try {
      const res = await fetch("/api/redirecter", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: jsonPreview,
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      toast.success("Redirecter config saved");
      setTouched(false);
    } catch {
      toast.error("Failed to save the Redirecter config");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <SectionHeading
        title="Redirecter"
        description="The configuration editor that generates owen.json, the file libowen.so reads at runtime."
      />

      <div className="grid gap-4 xl:grid-cols-2">
        <div className="flex flex-col gap-4">
          {/* Server */}
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="text-base text-zinc-100">Server</CardTitle>
              <CardDescription className="text-zinc-500">
                HTTP and HTTPS are both first-class: the scheme is kept verbatim, never rewritten.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="base-url" className="text-zinc-300">Server base URL</Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="base-url"
                    value={config.server.base_url}
                    onChange={(e) => setServer("base_url", e.target.value)}
                    placeholder="http://192.168.1.10:8000 or https://fn.example.org"
                    className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100 placeholder:text-zinc-600 focus-visible:ring-emerald-400/40"
                    aria-invalid={Boolean(baseError)}
                    autoComplete="off"
                    spellCheck={false}
                  />
                  {scheme ? (
                    <Badge
                      className={`shrink-0 border-emerald-400/40 bg-emerald-400/10 hover:bg-emerald-400/10 ${
                        scheme === "https" ? "text-emerald-400" : "text-emerald-400"
                      }`}
                    >
                      {scheme}
                    </Badge>
                  ) : null}
                </div>
                {baseError ? (
                  <p className="text-xs text-red-400" role="alert">{baseError}</p>
                ) : (
                  <p className="text-xs text-zinc-500">
                    {scheme
                      ? `Scheme detected: ${scheme}. It will be used exactly as entered.`
                      : "Enter http:// or https:// followed by the host."}
                  </p>
                )}
              </div>
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="xmpp-host" className="text-zinc-300">XMPP host</Label>
                  <Input
                    id="xmpp-host"
                    value={config.server.xmpp_host}
                    onChange={(e) => setServer("xmpp_host", e.target.value)}
                    placeholder="Defaults to the base URL host"
                    className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100 placeholder:text-zinc-600"
                    autoComplete="off"
                    spellCheck={false}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="xmpp-port" className="text-zinc-300">XMPP port</Label>
                  <Input
                    id="xmpp-port"
                    type="number"
                    value={config.server.xmpp_port}
                    onChange={(e) => setServer("xmpp_port", Number(e.target.value) || 0)}
                    className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100"
                    min={1}
                    max={65535}
                  />
                </div>
              </div>
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="login-path" className="text-zinc-300">Login path</Label>
                  <Input
                    id="login-path"
                    value={config.server.login_path}
                    onChange={(e) => setServer("login_path", e.target.value)}
                    className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100"
                    autoComplete="off"
                    spellCheck={false}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="graphql-path" className="text-zinc-300">GraphQL path</Label>
                  <Input
                    id="graphql-path"
                    value={config.server.graphql_path}
                    onChange={(e) => setServer("graphql_path", e.target.value)}
                    className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100"
                    autoComplete="off"
                    spellCheck={false}
                  />
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Redirect domains */}
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="text-base text-zinc-100">Redirect domains</CardTitle>
              <CardDescription className="text-zinc-500">
                Requests to these Epic domains are redirected using suffix matching.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {config.redirect.domains.length > 8 ? (
                <div className="max-h-96 overflow-y-auto pr-1">
                  <DomainList
                    domains={config.redirect.domains}
                    onRemove={(d) =>
                      set("redirect", {
                        ...config.redirect,
                        domains: config.redirect.domains.filter((x) => x !== d),
                      })
                    }
                  />
                </div>
              ) : (
                <DomainList
                  domains={config.redirect.domains}
                  onRemove={(d) =>
                    set("redirect", {
                      ...config.redirect,
                      domains: config.redirect.domains.filter((x) => x !== d),
                    })
                  }
                />
              )}
              <AddDomainInput
                onAdd={(d) => {
                  if (config.redirect.domains.includes(d)) {
                    toast.error("Domain already in the list");
                    return;
                  }
                  set("redirect", {
                    ...config.redirect,
                    domains: [...config.redirect.domains, d],
                  });
                }}
              />
            </CardContent>
          </Card>

          {/* Extra rules */}
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="text-base text-zinc-100">Extra rules</CardTitle>
              <CardDescription className="text-zinc-500">
                Repeatable match pattern to replace-path rules.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {config.redirect.extra_rules.length === 0 ? (
                <p className="text-sm text-zinc-500">No extra rules configured.</p>
              ) : (
                <div className="max-h-96 space-y-2 overflow-y-auto pr-1">
                  {config.redirect.extra_rules.map((rule, i) => (
                    <div
                      key={i}
                      className="flex flex-wrap items-center gap-2 rounded-md border border-zinc-800 bg-zinc-950/60 p-2"
                    >
                      <Input
                        value={rule.match}
                        aria-label={`Rule ${i + 1} match pattern`}
                        placeholder="match pattern"
                        onChange={(e) => {
                          const next = [...config.redirect.extra_rules];
                          next[i] = { ...next[i], match: e.target.value };
                          set("redirect", { ...config.redirect, extra_rules: next });
                        }}
                        className="h-9 min-w-0 flex-1 border-zinc-700 bg-zinc-950 font-mono text-xs text-zinc-100 placeholder:text-zinc-600"
                        autoComplete="off"
                        spellCheck={false}
                      />
                      <span className="text-zinc-600" aria-hidden="true">→</span>
                      <Input
                        value={rule.replace_path}
                        aria-label={`Rule ${i + 1} replace path`}
                        placeholder="/replace/path"
                        onChange={(e) => {
                          const next = [...config.redirect.extra_rules];
                          next[i] = { ...next[i], replace_path: e.target.value };
                          set("redirect", { ...config.redirect, extra_rules: next });
                        }}
                        className="h-9 min-w-0 flex-1 border-zinc-700 bg-zinc-950 font-mono text-xs text-zinc-100 placeholder:text-zinc-600"
                        autoComplete="off"
                        spellCheck={false}
                      />
                      <label className="flex h-9 min-w-20 items-center justify-center gap-2 text-xs text-zinc-400">
                        <Switch
                          checked={rule.enabled !== false}
                          onCheckedChange={(v) => {
                            const next = [...config.redirect.extra_rules];
                            next[i] = { ...next[i], enabled: v };
                            set("redirect", { ...config.redirect, extra_rules: next });
                          }}
                          className="data-[state=checked]:bg-emerald-400"
                          aria-label={`Enable rule ${i + 1}`}
                        />
                        on
                      </label>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-9 text-zinc-500 hover:text-red-400"
                        aria-label={`Remove rule ${i + 1}`}
                        onClick={() =>
                          set("redirect", {
                            ...config.redirect,
                            extra_rules: config.redirect.extra_rules.filter((_, j) => j !== i),
                          })
                        }
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
              <Button
                variant="outline"
                onClick={() =>
                  set("redirect", {
                    ...config.redirect,
                    extra_rules: [...config.redirect.extra_rules, { match: "", replace_path: "", enabled: true }],
                  })
                }
                className="border-zinc-700 bg-zinc-950 text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100"
              >
                <Plus className="size-4" /> Add rule
              </Button>
            </CardContent>
          </Card>
        </div>

        <div className="flex flex-col gap-4">
          {/* Manifest + webview + detect + hooks */}
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="text-base text-zinc-100">Manifest and webview</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="manifest-mode" className="text-zinc-300">Manifest mode</Label>
                <Select
                  value={config.manifest.mode}
                  onValueChange={(v) => set("manifest", { mode: v as "auto" | "off" })}
                >
                  <SelectTrigger id="manifest-mode" className="border-zinc-700 bg-zinc-950 text-zinc-100">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="border-zinc-700 bg-zinc-900 text-zinc-100">
                    <SelectItem value="auto">auto — manifest and asset CDN URLs are rewritten to the backend automatically</SelectItem>
                    <SelectItem value="off">off — leave manifest and CDN URLs untouched</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="login-url" className="text-zinc-300">Login URL override (optional)</Label>
                <Input
                  id="login-url"
                  value={config.webview.login_url}
                  onChange={(e) =>
                    set("webview", { ...config.webview, login_url: e.target.value })
                  }
                  placeholder="Leave empty to use base_url + login_path"
                  className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100 placeholder:text-zinc-600"
                  autoComplete="off"
                  spellCheck={false}
                />
              </div>
              <div className="flex items-center justify-between gap-4 rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
                <div>
                  <Label htmlFor="force-cleartext" className="text-zinc-200">Force cleartext HTTP</Label>
                  <p className="mt-0.5 text-xs text-zinc-500">
                    Unlocks the Android NetworkSecurityPolicy so plain-HTTP login pages work.
                  </p>
                </div>
                <Switch
                  id="force-cleartext"
                  checked={config.webview.force_cleartext}
                  onCheckedChange={(v) => set("webview", { ...config.webview, force_cleartext: v })}
                  className="data-[state=checked]:bg-emerald-400"
                  aria-label="Force cleartext HTTP"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="partyhub-mode" className="text-zinc-300">PartyHub mode</Label>
                <Select
                  value={config.webview.partyhub}
                  onValueChange={(v) => set("webview", { ...config.webview, partyhub: v as "auto" | "off" })}
                >
                  <SelectTrigger id="partyhub-mode" className="border-zinc-700 bg-zinc-950 text-zinc-100">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="border-zinc-700 bg-zinc-900 text-zinc-100">
                    <SelectItem value="auto">auto — ART hooks on the Kairos URL providers</SelectItem>
                    <SelectItem value="off">off — do not touch PartyHub URLs</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="text-base text-zinc-100">Detection and hooks</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="detect-strategy" className="text-zinc-300">Detect strategy</Label>
                <Select
                  value={config.detect.strategy}
                  onValueChange={(v) =>
                    set("detect", { ...config.detect, strategy: v as OwenConfig["detect"]["strategy"] })
                  }
                >
                  <SelectTrigger id="detect-strategy" className="border-zinc-700 bg-zinc-950 text-zinc-100">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="border-zinc-700 bg-zinc-900 text-zinc-100">
                    <SelectItem value="auto">auto — exported symbols first, then signature scan</SelectItem>
                    <SelectItem value="symbols-only">symbols-only — exported dynsym lookup only</SelectItem>
                    <SelectItem value="signature-only">signature-only — structural scan only</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="min-score" className="text-zinc-300">Minimum score</Label>
                <Input
                  id="min-score"
                  type="number"
                  min={0}
                  max={13}
                  value={config.detect.min_score}
                  onChange={(e) => set("detect", { ...config.detect, min_score: Number(e.target.value) || 0 })}
                  className="w-24 border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100"
                />
                <p className="text-xs text-zinc-500">
                  Candidates below this score are ignored at runtime (default 6, ground-truth hits score 13).
                </p>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="flex items-center justify-between gap-3 rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
                  <Label htmlFor="hook-curl" className="text-zinc-200">Hook curl</Label>
                  <Switch
                    id="hook-curl"
                    checked={config.hooks.curl}
                    onCheckedChange={(v) => set("hooks", { ...config.hooks, curl: v })}
                    className="data-[state=checked]:bg-emerald-400"
                    aria-label="Hook curl"
                  />
                </div>
                <div className="flex items-center justify-between gap-3 rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
                  <Label htmlFor="hook-art" className="text-zinc-200">Hook ART webview</Label>
                  <Switch
                    id="hook-art"
                    checked={config.hooks.art_webview}
                    onCheckedChange={(v) => set("hooks", { ...config.hooks, art_webview: v })}
                    className="data-[state=checked]:bg-emerald-400"
                    aria-label="Hook ART webview"
                  />
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Live JSON preview */}
          <Card className="border-zinc-800 bg-zinc-900">
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center justify-between gap-2 text-base text-zinc-100">
                <span className="flex items-center gap-2">
                  <FileJson className="size-4 text-emerald-400" aria-hidden="true" />
                  owen.json preview
                </span>
                {touched ? (
                  <Badge className="border-amber-400/30 bg-amber-400/10 text-[10px] text-amber-400 hover:bg-amber-400/10">
                    unsaved changes
                  </Badge>
                ) : null}
              </CardTitle>
              <CardDescription className="text-zinc-500">
                Updates live as you type. This exact object is read by libowen.so.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <pre
                className="max-h-96 overflow-auto rounded-md border border-zinc-800 bg-zinc-950 p-4 font-mono text-xs leading-relaxed text-zinc-300"
                aria-label="owen.json live preview"
              >
                {jsonPreview}
              </pre>
              <div className="flex flex-wrap gap-2">
                <Button
                  onClick={downloadJson}
                  className="bg-emerald-400 text-zinc-950 hover:bg-emerald-300"
                >
                  <Download className="size-4" /> Download owen.json
                </Button>
                <Button
                  variant="outline"
                  onClick={save}
                  disabled={saving || Boolean(baseError)}
                  className="border-zinc-700 bg-zinc-950 text-zinc-200 hover:bg-zinc-800"
                >
                  {saving ? <Loader2 className="size-4 animate-spin" /> : null}
                  Save to console
                </Button>
                {loading ? (
                  <span className="flex items-center gap-2 text-xs text-zinc-500">
                    <Loader2 className="size-3 animate-spin" /> loading saved config
                  </span>
                ) : null}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

function DomainList({ domains, onRemove }: { domains: string[]; onRemove: (d: string) => void }) {
  return (
    <ul className="space-y-2">
      {domains.map((d) => (
        <li
          key={d}
          className="flex items-center justify-between gap-2 rounded-md border border-zinc-800 bg-zinc-950/60 px-3 py-2"
        >
          <span className="truncate font-mono text-xs text-zinc-200">{d}</span>
          <Button
            variant="ghost"
            size="icon"
            className="size-8 shrink-0 text-zinc-500 hover:text-red-400"
            aria-label={`Remove ${d}`}
            onClick={() => onRemove(d)}
          >
            <Trash2 className="size-4" />
          </Button>
        </li>
      ))}
    </ul>
  );
}

function AddDomainInput({ onAdd }: { onAdd: (d: string) => void }) {
  const [value, setValue] = useState("");
  return (
    <form
      className="flex gap-2"
      onSubmit={(e) => {
        e.preventDefault();
        const v = value.trim();
        if (!v) return;
        onAdd(v);
        setValue("");
      }}
    >
      <Input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="add domain (suffix match)"
        aria-label="Add redirect domain"
        className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100 placeholder:text-zinc-600"
        autoComplete="off"
        spellCheck={false}
      />
      <Button type="submit" variant="outline" className="border-zinc-700 bg-zinc-950 text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100">
        <Plus className="size-4" /> Add
      </Button>
    </form>
  );
}

// ---------------------------------------------------------------------------
// Symbol Finder section
// ---------------------------------------------------------------------------

function SymbolFinderSection() {
  const [dragOver, setDragOver] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [result, setResult] = useState<DetectResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const scan = useCallback(async (file: File) => {
    if (file.size > 400 * 1024 * 1024) {
      toast.error("File exceeds the 400 MB limit");
      return;
    }
    setUploading(true);
    setProgress(0);
    setError(null);
    setResult(null);
    setFileName(file.name);
    try {
      const res = await new Promise<Response>((resolve, reject) => {
        const xhr = new XMLHttpRequest();
        xhr.open("POST", `/api/detect?name=${encodeURIComponent(file.name)}`);
        xhr.setRequestHeader("Content-Type", "application/octet-stream");
        xhr.upload.onprogress = (e) => {
          if (e.lengthComputable) setProgress(Math.round((e.loaded / e.total) * 100));
        };
        xhr.onload = () => resolve(new Response(xhr.response, { status: xhr.status, headers: { "Content-Type": "application/json" } }));
        xhr.onerror = () => reject(new Error("Network error during upload"));
        xhr.send(file);
      });
      const json = await res.json();
      if (!res.ok) {
        throw new Error(json?.error || `Scan failed (HTTP ${res.status})`);
      }
      setResult(json as DetectResponse);
      toast.success(
        json.best
          ? `Best candidate ${json.best.va ?? "n/a"} (score ${json.best.score}/13)`
          : "Scan complete — no candidates found"
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Scan failed";
      setError(msg);
      toast.error(msg);
    } finally {
      setUploading(false);
    }
  }, []);

  return (
    <div>
      <SectionHeading
        title="Symbol Finder"
        description="Upload a stripped ARM64 shared object; the scanner locates curl_easy_setopt without any symbol table."
      />

      <Card
        className={`border-dashed transition-colors ${
          dragOver ? "border-emerald-400 bg-emerald-400/5" : "border-zinc-700 bg-zinc-900"
        }`}
      >
        <CardContent className="p-6">
          <div
            role="button"
            tabIndex={0}
            aria-label="Upload a .so file for scanning (up to 400 MB)"
            className="flex min-h-44 cursor-pointer flex-col items-center justify-center gap-3 rounded-md outline-none focus-visible:ring-2 focus-visible:ring-emerald-400/50"
            onClick={() => inputRef.current?.click()}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                inputRef.current?.click();
              }
            }}
            onDragOver={(e) => {
              e.preventDefault();
              setDragOver(true);
            }}
            onDragLeave={() => setDragOver(false)}
            onDrop={(e) => {
              e.preventDefault();
              setDragOver(false);
              const file = e.dataTransfer.files?.[0];
              if (file) void scan(file);
            }}
          >
            {uploading ? (
              <>
                <Loader2 className="size-10 animate-spin text-emerald-400" aria-hidden="true" />
                <p className="text-sm text-zinc-300">
                  Uploading and scanning <span className="font-mono">{fileName}</span>...
                </p>
                <div className="w-full max-w-md">
                  <Progress value={progress} className="bg-zinc-800 [&>div]:bg-emerald-400" />
                  <p className="mt-1 text-center text-xs text-zinc-500">{progress}%</p>
                </div>
              </>
            ) : (
              <>
                <Upload className="size-10 text-zinc-500" aria-hidden="true" />
                <p className="text-sm font-medium text-zinc-200">
                  Drag and drop a .so file, or click to browse
                </p>
                <p className="text-xs text-zinc-500">
                  ARM64 ELF shared object, up to 400 MB, sent as a raw binary stream
                </p>
              </>
            )}
          </div>
          <input
            ref={inputRef}
            type="file"
            accept=".so,application/octet-stream"
            className="sr-only"
            aria-label="Choose a .so file"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void scan(file);
              e.target.value = "";
            }}
          />
        </CardContent>
      </Card>

      {error ? (
        <Card className="mt-4 border-red-400/30 bg-red-400/5">
          <CardContent className="p-4 text-sm text-red-400" role="alert">{error}</CardContent>
        </Card>
      ) : null}

      {result ? (
        <Card className="mt-4 border-zinc-800 bg-zinc-900">
          <CardHeader className="pb-3">
            <CardTitle className="text-base text-zinc-100">Results</CardTitle>
            <CardDescription className="text-zinc-500">
              {result.fileName} — {(result.sizeBytes / 1048576).toFixed(1)} MB scanned in{" "}
              {(result.elapsedMs / 1000).toFixed(2)} s. Top {result.candidates.length} candidate
              {result.candidates.length === 1 ? "" : "s"}, sorted by score.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="max-h-96 overflow-auto pr-1">
              <Table>
                <TableHeader className="sticky top-0 bg-zinc-900">
                  <TableRow className="border-zinc-800 hover:bg-transparent">
                    <TableHead className="text-zinc-400">Score</TableHead>
                    <TableHead className="text-zinc-400">Fingerprint</TableHead>
                    <TableHead className="text-zinc-400">Function start (VA / file offset)</TableHead>
                    <TableHead className="text-zinc-400">CBZ X0</TableHead>
                    <TableHead className="text-zinc-400">BL → target</TableHead>
                    <TableHead className="text-zinc-400">MOV W0,#0x2b</TableHead>
                    <TableHead className="text-zinc-400">Gap</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {result.candidates.map((c, i) => (
                    <TableRow
                      key={i}
                      className={`border-zinc-800 ${
                        i === 0 ? "bg-emerald-400/10 hover:bg-emerald-400/10" : ""
                      }`}
                    >
                      <TableCell className="font-mono text-zinc-100">
                        <span className="inline-flex items-center gap-1.5">
                          {c.score}/13
                          {i === 0 ? (
                            <Badge className="border-emerald-400/40 bg-emerald-400/10 text-[10px] text-emerald-400 hover:bg-emerald-400/10">
                              best
                            </Badge>
                          ) : null}
                        </span>
                      </TableCell>
                      <TableCell>
                        {c.variadic ? (
                          <Badge className="border-emerald-400/30 bg-emerald-400/10 text-[10px] text-emerald-400 hover:bg-emerald-400/10">
                            va_start fingerprint
                          </Badge>
                        ) : (
                          <span className="text-xs text-zinc-600">—</span>
                        )}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-emerald-400">
                        <span className="inline-flex flex-wrap items-center gap-0.5">
                          <span className="inline-flex items-center gap-0.5">
                            {c.va ?? "n/a"}
                            {c.va ? <CopyButton value={c.va} label={`VA ${c.va}`} /> : null}
                          </span>
                          {c.funcStart ? (
                            <span className="ml-1 text-[10px] text-zinc-500">
                              (off {c.funcStart})
                            </span>
                          ) : null}
                        </span>
                      </TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">{c.cbzX0 ?? "—"}</TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">
                        {c.bl !== null && c.bl !== undefined ? (
                          <span>
                            {c.bl} <span className="text-zinc-600">→</span> {c.blTarget ?? "?"}
                          </span>
                        ) : (
                          "—"
                        )}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">{c.movW02b ?? "—"}</TableCell>
                      <TableCell className="font-mono text-xs text-zinc-400">{c.gap}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>
      ) : null}

      <Card className="mt-4 border-zinc-800 bg-zinc-900">
        <CardContent className="p-0">
          <Collapsible>
            <CollapsibleTrigger className="flex w-full items-center justify-between p-4 text-left text-sm font-medium text-zinc-200 hover:text-emerald-400">
              How it works — the four scan layers and the v2 improvements
              <ChevronDown className="size-4 text-zinc-500" aria-hidden="true" />
            </CollapsibleTrigger>
            <CollapsibleContent className="px-4 pb-4">
              <ol className="space-y-3 text-sm leading-relaxed text-zinc-400">
                <li>
                  <span className="font-medium text-zinc-200">Layer 1 — the error constant.</span>{" "}
                  <span className="font-mono text-emerald-400">MOV W0, #0x2b</span> loads
                  CURLE_BAD_FUNCTION_ARGUMENT (43), the exact value{" "}
                  <span className="font-mono">curl_easy_setopt</span> returns when the handle
                  argument is NULL.
                </li>
                <li>
                  <span className="font-medium text-zinc-200">Layer 2 — the NULL check.</span> A{" "}
                  <span className="font-mono text-emerald-400">CBZ X0</span> instruction whose
                  branch target is that MOV: the classic &quot;if (handle == NULL) return 43&quot;
                  guard.
                </li>
                <li>
                  <span className="font-medium text-zinc-200">Layer 3 — the forwarding call.</span>{" "}
                  A <span className="font-mono text-emerald-400">BL</span> to Curl_vsetopt
                  between the CBZ and the MOV — the variadic core that curl_easy_setopt forwards
                  to.
                </li>
                <li>
                  <span className="font-medium text-zinc-200">Layer 4 — the prologue walk.</span>{" "}
                  A backward walk from the CBZ (window 0x100) to the function prologue yields the
                  hook address.
                </li>
              </ol>
              <Separator className="my-4 bg-zinc-800" />
              <p className="mb-2 text-xs font-medium uppercase tracking-wide text-zinc-500">
                v2 improvements
              </p>
              <ul className="list-inside space-y-2 text-sm leading-relaxed text-zinc-400">
                <li>
                  <span className="text-zinc-200">Closest-marker backward walk</span> — the
                  closest prologue marker wins, instead of the earliest forward marker (the
                  reference algorithm picked a marker inside the previous function on libUE4
                  12.50, off by 0x68 bytes).
                </li>
                <li>
                  <span className="text-zinc-200">Widened markers</span> — any{" "}
                  <span className="font-mono">stp Xt1,Xt2,[sp,#-N]!</span> counts as a prologue,
                  matching how clang emits <span className="font-mono">stp x20,x19</span> first.
                </li>
                <li>
                  <span className="text-zinc-200">Predecessor sanity check</span> — the word
                  before a true function start is almost always ret / b / bl / b.cond /
                  ldp-post-index / nop, so markers preceded by ordinary arithmetic are skipped.
                </li>
                <li>
                  <span className="text-zinc-200">Variadic +3 fingerprint bonus</span> — the
                  prologue of a variadic function spills q0-q7 as STP Q pairs; two or more is
                  the va_start fingerprint, near-impossible to hit by accident.
                </li>
              </ul>
            </CollapsibleContent>
          </Collapsible>
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PartyHub and Login section
// ---------------------------------------------------------------------------

const VERSION_MATRIX: Array<{
  range: string;
  era: string;
  method: string;
  note?: string;
}> = [
  {
    range: "10.31 – 12.60",
    era: "Kairos era",
    method:
      "ART runtime hooks on DebouncedLimiter.loginUrl(Environment,boolean) and getGraphqlUrl(Environment) + WebView.loadUrl rewrite + curl_easy_setopt hook on libUE4.so (exported symbol; also located statically by owen-detect).",
  },
  {
    range: "12.61 – 15.50",
    era: "Kairos PartyHub era",
    method:
      "Same ART hooks (loginUrl + getGraphqlUrl + WebView.loadUrl) — this replaces the old smali const-string patching entirely (no smali edits at all).",
  },
  {
    range: "13.40 – 21.30",
    era: "libUnreal.so era",
    method:
      "owen-detect signature scan finds curl_easy_setopt in the stripped binary (no offset tables needed anymore — the old Owen.c required a per-version hardcoded table for 18.40+ and had NO support below 18.40).",
  },
  {
    range: "22.00 – 33.00+",
    era: "libUnreal.so + libEOSSDK.so",
    method:
      "Same signature scan across both libraries; both are hooked.",
  },
];

const NO_SMALI_LAYERS: Array<{ title: string; body: string }> = [
  {
    title: "Layer 1 — curl hook",
    body: "All engine HTTP traffic (libUE4.so / libUnreal.so / libEOSSDK.so) flows through curl_easy_setopt; hooking it redirects every request to the backend.",
  },
  {
    title: "Layer 2 — ART hooks",
    body: "WebView.loadUrl and the Kairos URL providers (DebouncedLimiter.loginUrl, getGraphqlUrl) are hooked in the ART runtime, covering the login page and PartyHub.",
  },
  {
    title: "Layer 3 — cleartext unlock",
    body: "NetworkSecurityPolicy.isCleartextTrafficPermitted is forced to true so plain-HTTP login pages work behind the hook.",
  },
  {
    title: "Layer 4 — auto manifest redirect",
    body: "Manifest and asset CDN URLs are rewritten to the backend automatically when manifest mode is auto.",
  },
];

interface PartyHubHit {
  time: string;
  method: string;
  path: string;
  op: string;
  via: "voltronite" | "local" | "-";
  detail: string;
}

interface PartyHubStatus {
  ok: boolean;
  voltronite: string;
  user: string;
  accountId: string;
  hits: PartyHubHit[];
}

/**
 * The runnable phone package (v3): golden launcher login, no debug strip,
 * Voltronite-compatible links in owen-bridge.js. Download + release notes.
 */
function PhonePackageCard() {
  return (
    <Card className="mt-4 border-amber-400/30 bg-gradient-to-b from-amber-400/10 to-zinc-900">
      <CardHeader className="pb-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <CardTitle className="flex items-center gap-2 text-base text-amber-300">
            <span
              className="rounded px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-900"
              style={{ background: "linear-gradient(90deg,#FFB300,#FF9F0A,#FF8A00)" }}
            >
              v3
            </span>
            Phone package — Owen PartyHub (Voltronite edition)
          </CardTitle>
          <a href="/owen/Owen-PartyHub-Voltronite.7z" download>
            <Button
              size="sm"
              className="h-8 border-0 text-xs font-bold text-zinc-900"
              style={{ background: "linear-gradient(90deg,#FFB300,#FF9F0A,#FF8A00)" }}
            >
              Download .7z (3.4 MB)
            </Button>
          </a>
        </div>
        <CardDescription className="text-zinc-500">
          Run it directly on the phone: extract, <span className="font-mono text-zinc-400">python server.py</span>{" "}
          (Termux), open http://127.0.0.1:8081. The Voltronite address is edited in one place —{" "}
          <span className="font-mono text-zinc-400">owen-bridge.js</span> →{" "}
          <span className="font-mono text-zinc-400">OWEN_CONFIG.VOLTRONITE_URL</span>.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-2 sm:grid-cols-3">
          {[
            {
              title: "No top strip",
              body: "The injected Arabic debug overlay (the red-on-black strip at the top of every screen) is stripped from index.html by the patcher — verified pixel-level: zero red pixels in the top rows.",
            },
            {
              title: "Golden launcher login",
              body: "The login page is the Owen Launcher Aurum design: charcoal background, OWEN / LAUNCHER gold-gradient wordmark, player-name card with the Required/OK pill and the gold LOGIN button. Logout returns to it.",
            },
            {
              title: "Voltronite links",
              body: "owen-bridge.js v3 maps every raw Kairos query to a Voltronite operation (GetMyAccount, GetStatusPerService, GetMySetting, …) and relays it with a real oauth token. Account ids are zero-padded to 32 chars so short names can't crash the home screen.",
            },
          ].map((f) => (
            <div key={f.title} className="rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
              <p className="text-xs font-semibold text-amber-200">{f.title}</p>
              <p className="mt-1 text-[11px] leading-relaxed text-zinc-500">{f.body}</p>
            </div>
          ))}
        </div>
        <p className="mt-3 text-[11px] leading-relaxed text-zinc-500">
          Package contents: <span className="font-mono text-zinc-400">server.py</span> (zero-dependency
          static + SPA + Voltronite relay), the patched{" "}
          <span className="font-mono text-zinc-400">offline/entry</span> build, and an Arabic README with
          Termux instructions. E2E-verified on this machine: golden login → LOGIN → PartyHub home
          renders, and Voltronite answers GetMyAccount / GetMySetting / GetAccountSettings /
          UpdateUserSetting for the typed account name.
        </p>
      </CardContent>
    </Card>
  );
}

/**
 * Live embed of the patched shippedweb Party Hub. The Kairos web itself runs in
 * mini-services/partyhub-web (:8091) behind the /partyhub rewrite; its raw
 * graphql dialect is adapted to the Voltronite backend (see the card below).
 */
function LivePartyHubDemo() {
  const [status, setStatus] = useState<PartyHubStatus | null>(null);
  const [alive, setAlive] = useState<boolean | null>(null);
  const [nonce, setNonce] = useState(0);
  const voltroniteHits = (status?.hits ?? []).filter((h) => h.via === "voltronite").length;

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const res = await fetch(`/api/partyhub-status?_=${Date.now()}`, { cache: "no-store" });
        if (!res.ok) throw new Error(String(res.status));
        const data: PartyHubStatus = await res.json();
        if (!cancelled) {
          setStatus(data);
          setAlive(true);
        }
      } catch {
        if (!cancelled) setAlive(false);
      }
    };
    poll();
    const t = setInterval(poll, 4000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  return (
    <Card className="mt-4 border-emerald-400/40 bg-gradient-to-b from-emerald-400/10 to-zinc-900">
      <CardHeader className="pb-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <CardTitle className="flex items-center gap-2 text-base text-emerald-300">
            <span className="relative flex size-2.5">
              <span
                className={`absolute inline-flex h-full w-full animate-ping rounded-full ${
                  alive ? "bg-emerald-400" : "bg-red-400"
                } opacity-60`}
              />
              <span
                className={`relative inline-flex size-2.5 rounded-full ${
                  alive ? "bg-emerald-400" : "bg-red-500"
                }`}
              />
            </span>
            LIVE — shippedweb Party Hub on Voltronite
          </CardTitle>
          <div className="flex items-center gap-2">
            <Badge
              variant="outline"
              className={`border-zinc-700 font-mono text-[11px] ${
                alive ? "text-emerald-300" : "text-red-300"
              }`}
            >
              {alive === null ? "checking…" : alive ? "partyhub-web :8091 online" : "offline"}
            </Badge>
            <Button
              variant="outline"
              size="sm"
              className="h-7 border-zinc-700 px-2 text-xs text-zinc-300"
              onClick={() => setNonce((n) => n + 1)}
            >
              Reload embed
            </Button>
          </div>
        </div>
        <CardDescription className="text-zinc-500">
          The real Kairos bundle (build 8af1b828) patched to mount the full Party Hub app — served
          by partyhub-web, authenticated against the Voltronite backend.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4 lg:grid-cols-[380px_1fr]">
          {/* phone frame */}
          <div className="mx-auto w-full max-w-[380px]">
            <div className="relative overflow-hidden rounded-[28px] border-4 border-zinc-700 bg-black shadow-2xl">
              <div className="flex h-6 items-center justify-center bg-zinc-900">
                <div className="h-1.5 w-16 rounded-full bg-zinc-700" />
              </div>
              <iframe
                key={nonce}
                src="/partyhub/offline/en-US"
                title="Fortnite Party Hub (patched shippedweb)"
                className="block h-[620px] w-full bg-black"
                sandbox="allow-scripts allow-same-origin allow-forms"
              />
            </div>
            <p className="mt-2 text-center text-xs text-zinc-500">
              The golden <span className="text-amber-300">OWEN LAUNCHER</span> login page — type a
              name, press <span className="text-zinc-300">LOGIN</span>, and the hub mounts with that
              Voltronite account (accountId = hex(name)).
            </p>
          </div>

          {/* backend panel */}
          <div className="min-w-0 space-y-3">
            <div className="grid grid-cols-3 gap-2">
              <div className="rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
                <p className="text-[11px] uppercase tracking-wide text-zinc-500">Backend</p>
                <p className="mt-1 truncate font-mono text-sm text-zinc-200">Voltronite</p>
              </div>
              <div className="rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
                <p className="text-[11px] uppercase tracking-wide text-zinc-500">Account</p>
                <p className="mt-1 truncate font-mono text-sm text-zinc-200">
                  {status?.user ?? "—"}
                </p>
              </div>
              <div className="rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
                <p className="text-[11px] uppercase tracking-wide text-zinc-500">Requests → backend</p>
                <p className="mt-1 font-mono text-sm text-emerald-300">
                  {voltroniteHits} (last 60)
                </p>
              </div>
            </div>
            <div>
              <p className="mb-1.5 text-xs font-medium text-zinc-400">
                GraphQL request log (recent, newest first)
              </p>
              <div className="max-h-72 overflow-y-auto rounded-md border border-zinc-800 bg-zinc-950/80">
                {status && status.hits.length > 0 ? (
                  <table className="w-full text-left text-xs">
                    <thead className="sticky top-0 bg-zinc-900 text-zinc-500">
                      <tr>
                        <th className="px-2 py-1.5 font-medium">time</th>
                        <th className="px-2 py-1.5 font-medium">operation</th>
                        <th className="px-2 py-1.5 font-medium">via</th>
                      </tr>
                    </thead>
                    <tbody>
                      {status.hits.map((h, i) => (
                        <tr key={`${h.time}-${i}`} className="border-t border-zinc-800/70">
                          <td className="whitespace-nowrap px-2 py-1.5 font-mono text-zinc-500">
                            {h.time}
                          </td>
                          <td className="px-2 py-1.5 font-mono text-zinc-300">
                            {h.op.includes("GRAPHQL") ? h.op.replace("GRAPHQL ", "") : h.op}
                          </td>
                          <td className="whitespace-nowrap px-2 py-1.5">
                            <span
                              className={`rounded px-1.5 py-0.5 font-mono text-[10px] ${
                                h.via === "voltronite"
                                  ? "bg-emerald-400/10 text-emerald-300"
                                  : h.via === "local"
                                    ? "bg-zinc-700/40 text-zinc-400"
                                    : "bg-zinc-800 text-zinc-500"
                              }`}
                            >
                              {h.via === "voltronite" ? "voltronite" : h.via}
                            </span>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                ) : (
                  <p className="px-3 py-6 text-center text-xs text-zinc-500">
                    {alive
                      ? "No requests yet — open the embed on the left and log in."
                      : "partyhub-web is not reachable on :8091."}
                  </p>
                )}
              </div>
            </div>
            <p className="text-xs leading-relaxed text-zinc-500">
              Requests tagged <span className="font-mono text-emerald-300">voltronite</span> were
              forwarded to the real backend (
              <span className="font-mono">{status?.voltronite ?? "http://127.0.0.1:3551"}</span>):
              the adapter maps each raw Kairos query (no operationName, no auth header) to a
              Voltronite operation and attaches the oauth token it obtained from
              /account/api/oauth/token.
            </p>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function PartyHubSection() {
  return (
    <div>
      <SectionHeading
        title="PartyHub and Login"
        description="How the no-smali stack covers login and PartyHub across every game version."
      />

      <LivePartyHubDemo />

      <PhonePackageCard />

      <Card className="border-zinc-800 bg-zinc-900">
        <CardHeader className="pb-3">
          <CardTitle className="text-base text-zinc-100">Version matrix</CardTitle>
          <CardDescription className="text-zinc-500">
            One library, every era from 10.31 to 33.00+.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow className="border-zinc-800 hover:bg-transparent">
                <TableHead className="text-zinc-400">Game versions</TableHead>
                <TableHead className="text-zinc-400">Era</TableHead>
                <TableHead className="text-zinc-400">Method</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {VERSION_MATRIX.map((row) => (
                <TableRow key={row.range} className="border-zinc-800 align-top">
                  <TableCell className="whitespace-nowrap font-mono text-xs text-emerald-400">
                    {row.range}
                  </TableCell>
                  <TableCell className="whitespace-nowrap text-sm text-zinc-200">{row.era}</TableCell>
                  <TableCell className="text-sm leading-relaxed text-zinc-400">{row.method}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card className="mt-4 border-emerald-400/30 bg-emerald-400/5">
        <CardHeader className="pb-2">
          <CardTitle className="text-base text-emerald-300">
            shippedweb offline entry — from frozen login to the full Party Hub
          </CardTitle>
          <CardDescription className="text-zinc-500">
            Verified end to end against the real archive (Kairos build 8af1b828dc6188736f50) with
            the Voltronite backend.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm leading-relaxed text-zinc-300">
            <span className="font-medium text-zinc-100">Why it hung on “Checking your login info”.</span>{" "}
            Three independent causes, all fixed: (1) the built-in mock bridge answers{" "}
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-amber-300">
              getstartupstatus
            </code>{" "}
            with <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-amber-300">authenticated</code>{" "}
            — not a case in the LoginContainer switch, so the Epic logo never clears; (2) the entry
            build mounts <em>only</em> the LoginContainer as its root — the entire Party Hub router
            (home, party, friends, settings, modals) ships inside the same bundle but is never
            mounted, so “postlogin” is a dead-end screen by design; (3) the graphql singleton (
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-amber-300">
              window.__GraphQlService
            </code>
            ) is never configured in the offline build — every query POSTs to an empty URL (the
            page itself) and the account/friends initialization silently fails.
          </p>
          <p className="text-sm leading-relaxed text-zinc-300">
            <span className="font-medium text-zinc-100">The v2 fix (three patches).</span>{" "}
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-emerald-300">
              patch_partyhub.py
            </code>{" "}
            replaces webpack module 1041 with a hybrid root (login flow until authenticated, then
            the real app: router + AppEffects + useAppInitialization + AppStateInitializers), makes
            module 170 self-configure the GraphQlService from{" "}
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-emerald-300">
              window.clientEnvConfig
            </code>
            , and injects the stateful Owen bridge (login → loginchanged → authenticated and stop;
            getstartupstatus always reports the current status; getconfiguredbackend; seeded
            onboarding settings). Handled commands are consumed with
            stopImmediatePropagation so the built-in mock cannot navigate the page away.
          </p>
          <p className="text-sm leading-relaxed text-zinc-300">
            <span className="font-medium text-zinc-100">Voltronite dialect adapter (v3 — in owen-bridge.js).</span>{" "}
            The Kairos web POSTs raw graphql documents over axios/XMLHttpRequest (no operationName,
            no Authorization header); Voltronite switches on operationName and requires a Bearer
            token. The bridge hooks XHR directly: it fingerprints each query client-side, maps it
            to a Voltronite operation, and relays{" "}
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-emerald-300">
              /owen-graphql
            </code>{" "}
            (same-origin — browsers cannot call Voltronite directly, it sends no CORS headers) with
            the backend URL and account name in headers. The relay (server.py on the phone,
            partyhub-web here) attaches a real token from /account/api/oauth/token (password grant,
            cached per user, refreshed on 401) and forwards to /partyhub/graphql — the backend
            receives and answers GetMyAccount, GetStatusPerService, EulaAccepted, GetMySetting,
            GetAccountSettings, UpdateUserSetting, GetNotificationSettings. The bridge applies the
            compatibility transforms (kairos lightswitch DOWN→UP, short account ids zero-padded to
            32 chars so short names can't crash the home screen) and answers the queries Voltronite
            does not implement (friends lists, eula body, character profile) locally — with a local
            fallback also when the backend is unreachable, so the hub always renders.
          </p>
          <p className="text-sm leading-relaxed text-zinc-300">
            <span className="font-medium text-zinc-100">Verified in a real browser.</span> The
            golden launcher login renders → type a name → LOGIN → the router lands on{" "}
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-emerald-300">
              /offline/en-US/home
            </code>
            with the Party Hub home screen (onboarding cards, microphone and notification
            permissions, Llama Bell, friend-request badge, Add Friends and Share The App actions),
            modal routes work (universal search), logout returns to the golden login, and the
            request log above shows every forwarded operation for the typed account name. Patch any
            copy with{" "}
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-emerald-300">
              patch_partyhub.py &lt;shippedweb&gt; --out &lt;copy&gt; --base &lt;route-base&gt; --server
            </code>
            . Full protocol and serving rules:{" "}
            <code className="rounded bg-zinc-800 px-1 py-0.5 font-mono text-xs text-emerald-300">
              tools/shippedweb/PATCH-GUIDE.md
            </code>
            .
          </p>
        </CardContent>
      </Card>

      <Card className="mt-4 border-amber-400/30 bg-amber-400/5">
        <CardContent className="p-4">
          <p className="text-sm leading-relaxed text-amber-200/90">
            <span className="font-medium">
              Older APKs investigated (10.10, 12.50, 13.00) — no offline entry:
            </span>{" "}
            inspected real APKs end to end (assets + hidden main.obb.png payload). 12.50 (inside
            the claimed 10.31-12.60 range) and 13.00 ship only 7-8 asset files and their OBB
            contains a single pakchunk — the shippedweb/offline entry does not exist in those
            builds. The Kairos offline entry belongs to the newer login flow; if your APK contains
            assets/shippedweb/, the fix above applies to it directly. The ART runtime hooks
            (DebouncedLimiter + WebView.loadUrl) remain the correct smali-free path for the
            in-game login on every version.
          </p>
        </CardContent>
      </Card>

      <Card className="mt-4 border-zinc-800 bg-zinc-900">
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base text-zinc-100">
            <Layers className="size-4 text-emerald-400" aria-hidden="true" />
            How the no-smali stack works
          </CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="grid gap-3 sm:grid-cols-2">
            {NO_SMALI_LAYERS.map((layer, i) => (
              <li key={layer.title} className="rounded-md border border-zinc-800 bg-zinc-950/60 p-4">
                <p className="flex items-center gap-2 text-sm font-medium text-zinc-200">
                  <span className="flex size-6 shrink-0 items-center justify-center rounded-full border border-emerald-400/30 bg-emerald-400/10 text-[10px] font-semibold text-emerald-400">
                    {i + 1}
                  </span>
                  {layer.title}
                </p>
                <p className="mt-2 text-xs leading-relaxed text-zinc-500">{layer.body}</p>
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Battle Pass section
// ---------------------------------------------------------------------------

const SEASON_IMAGE_NUMBERS = [5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33];

interface RewardRow {
  type: string;
  id: string;
  quantity: number;
}

function splitRewards(rows: Array<Record<string, number>> | undefined): RewardRow[] {
  const out: RewardRow[] = [];
  for (const row of rows ?? []) {
    for (const [key, quantity] of Object.entries(row)) {
      const idx = key.indexOf(":");
      const type = idx === -1 ? "Token" : key.slice(0, idx);
      const id = idx === -1 ? key : key.slice(idx + 1);
      out.push({ type, id, quantity });
    }
  }
  return out;
}

function RewardList({ title, rows, badgeClass }: { title: string; rows: RewardRow[]; badgeClass: string }) {
  return (
    <div>
      <p className="mb-2 text-xs font-medium uppercase tracking-wide text-zinc-500">
        {title} ({rows.length})
      </p>
      {rows.length === 0 ? (
        <p className="text-sm text-zinc-500">No entries.</p>
      ) : (
        <ul className="max-h-96 space-y-1 overflow-y-auto pr-1">
          {rows.map((r, i) => (
            <li
              key={`${r.type}-${r.id}-${i}`}
              className="flex items-center gap-2 rounded border border-zinc-800 bg-zinc-950/60 px-2.5 py-1.5"
            >
              <Badge className={`shrink-0 border-zinc-700 bg-zinc-800 text-[10px] text-zinc-300 hover:bg-zinc-800 ${badgeClass}`}>
                {r.type}
              </Badge>
              <span className="min-w-0 flex-1 truncate font-mono text-xs text-zinc-300">{r.id}</span>
              <span className="shrink-0 font-mono text-xs text-zinc-500">x{r.quantity}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function BattlePassSection() {
  const [seasons, setSeasons] = useState<SeasonSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<SeasonDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [importOpen, setImportOpen] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/seasons");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      setSeasons(json);
    } catch {
      toast.error("Failed to load seasons");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const openSeason = async (seasonNumber: number) => {
    setDetailLoading(true);
    setSelected(null);
    try {
      const res = await fetch(`/api/seasons/${seasonNumber}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      setSelected(json);
    } catch {
      toast.error("Failed to load season details");
    } finally {
      setDetailLoading(false);
    }
  };

  const exportSeason = async (seasonNumber: number) => {
    try {
      const res = await fetch(`/api/seasons/${seasonNumber}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      const blob = new Blob([JSON.stringify(json.data, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `Season${seasonNumber}-reload-backend.json`;
      a.click();
      URL.revokeObjectURL(url);
      toast.success(`Season ${seasonNumber} exported in Reload-Backend shape`);
    } catch {
      toast.error("Export failed");
    }
  };

  return (
    <div>
      <SectionHeading
        title="Battle Pass"
        description="Seasons gallery with per-season reward tables extracted by owen-pak."
      />

      <div className="mb-4 flex flex-wrap gap-2">
        <Button
          onClick={() => setImportOpen(true)}
          className="bg-emerald-400 text-zinc-950 hover:bg-emerald-300"
        >
          <Upload className="size-4" /> Import
        </Button>
        <Button
          variant="outline"
          onClick={() => void load()}
          disabled={loading}
          className="border-zinc-700 bg-zinc-900 text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100"
        >
          {loading ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
          Refresh
        </Button>
        <a
          href="/api/seasons/catalog"
          download="battlepass-catalog-21plus.json"
          className="inline-flex h-9 items-center gap-2 rounded-md border border-zinc-700 bg-zinc-900 px-3 text-sm text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-zinc-100"
        >
          <Download className="size-4" /> Catalog 21+
        </a>
        <span className="inline-flex h-9 items-center rounded-md border border-zinc-800 bg-zinc-950 px-3 text-xs text-zinc-500">
          {seasons.length} seasons served — S2 through S{seasons.length ? seasons[seasons.length - 1].seasonNumber : "42"}
        </span>
      </div>

      {loading && seasons.length === 0 ? (
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4">
          {Array.from({ length: 8 }).map((_, i) => (
            <div key={i} className="aspect-square animate-pulse rounded-lg border border-zinc-800 bg-zinc-900" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
          {seasons.map((s) => {
            const hasArt = SEASON_IMAGE_NUMBERS.includes(s.seasonNumber);
            return (
              <button
                key={s.seasonNumber}
                onClick={() => void openSeason(s.seasonNumber)}
                className="group relative overflow-hidden rounded-lg border border-zinc-800 bg-zinc-900 outline-none transition-colors hover:border-emerald-400/60 focus-visible:ring-2 focus-visible:ring-emerald-400/50"
                aria-label={`Open Season ${s.seasonNumber} rewards`}
              >
                {hasArt ? (
                  <img
                    src={`/owen/seasons/season_${s.seasonNumber}_folder.png`}
                    alt={`Fortnite Season ${s.seasonNumber} battle pass folder art`}
                    className="aspect-square w-full object-cover transition-transform duration-300 group-hover:scale-105"
                    loading="lazy"
                    width={512}
                    height={512}
                  />
                ) : (
                  <div className="flex aspect-square w-full items-center justify-center bg-gradient-to-br from-zinc-900 via-zinc-950 to-zinc-900 transition-transform duration-300 group-hover:scale-105">
                    <span className="text-3xl font-bold tracking-tight text-zinc-700">
                      S{s.seasonNumber}
                    </span>
                  </div>
                )}
                <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-zinc-950/95 via-zinc-950/70 to-transparent p-2.5 pt-8 text-left">
                  <p className="text-sm font-semibold text-zinc-100">Season {s.seasonNumber}</p>
                  <p className="mt-0.5 text-[11px] text-zinc-400">
                    {s.paidCount} paid / {s.freeCount} free rewards
                  </p>
                </div>
                {s.hasBundle ? (
                  <span className="absolute right-2 top-2 rounded bg-emerald-400 px-1.5 py-0.5 text-[10px] font-semibold text-zinc-950">
                    bundle
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
      )}

      <Dialog open={selected !== null || detailLoading} onOpenChange={(open) => {
        if (!open) setSelected(null);
      }}>
        <DialogContent className="max-h-[85vh] overflow-y-auto border-zinc-800 bg-zinc-900 sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle className="text-zinc-100">
              {selected ? selected.displayName : "Loading season..."}
            </DialogTitle>
            <DialogDescription className="text-zinc-500">
              {selected
                ? `Paid and free reward tables. Updated ${formatDate(selected.updatedAt)}.`
                : ""}
            </DialogDescription>
          </DialogHeader>
          {detailLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="size-6 animate-spin text-emerald-400" aria-hidden="true" />
            </div>
          ) : selected ? (
            <div className="space-y-4">
              <div className="flex flex-wrap gap-2 text-xs">
                {selected.data.battlePassOfferId ? (
                  <span className="rounded border border-zinc-800 bg-zinc-950 px-2 py-1 font-mono text-zinc-400">
                    pass: {selected.data.battlePassOfferId}
                  </span>
                ) : null}
                {selected.data.tierOfferId ? (
                  <span className="rounded border border-zinc-800 bg-zinc-950 px-2 py-1 font-mono text-zinc-400">
                    tier: {selected.data.tierOfferId}
                  </span>
                ) : null}
                {selected.data.battleBundleOfferId ? (
                  <span className="rounded border border-zinc-800 bg-zinc-950 px-2 py-1 font-mono text-zinc-400">
                    bundle: {selected.data.battleBundleOfferId}
                  </span>
                ) : null}
              </div>
              <div className="grid gap-4 sm:grid-cols-2">
                <RewardList title="Paid rewards" rows={splitRewards(selected.data.paidRewards)} badgeClass="!text-emerald-400" />
                <RewardList title="Free rewards" rows={splitRewards(selected.data.freeRewards)} badgeClass="!text-zinc-400" />
              </div>
              <Button
                variant="outline"
                onClick={() => void exportSeason(selected.seasonNumber)}
                className="border-zinc-700 bg-zinc-950 text-zinc-200 hover:bg-zinc-800"
              >
                <Download className="size-4" /> Export Reload-Backend JSON
              </Button>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>

      <ImportDialog open={importOpen} onOpenChange={setImportOpen} onImported={load} />
    </div>
  );
}

function ImportDialog({
  open,
  onOpenChange,
  onImported,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImported: () => void;
}) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [seasonNumber, setSeasonNumber] = useState("");

  const submit = async () => {
    let data: unknown;
    try {
      data = JSON.parse(text);
    } catch {
      toast.error("Invalid JSON — paste the owen-pak output verbatim");
      return;
    }
    const n = Number(seasonNumber);
    if (!Number.isInteger(n) || n < 1 || n > 100) {
      toast.error("Season number must be an integer between 1 and 100");
      return;
    }
    setBusy(true);
    try {
      const res = await fetch("/api/seasons/import", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ seasonNumber: n, data }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json?.error || `HTTP ${res.status}`);
      toast.success(`Season ${n} imported`);
      onImported();
      onOpenChange(false);
      setText("");
      setSeasonNumber("");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Import failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85vh] overflow-y-auto border-zinc-800 bg-zinc-900 sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="text-zinc-100">Import season data</DialogTitle>
          <DialogDescription className="text-zinc-500">
            Paste the JSON produced by owen-pak (battleBundleOfferId, battlePassOfferId,
            tierOfferId, paidRewards, freeRewards).
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-2">
            <Label htmlFor="import-season" className="text-zinc-300">Season number</Label>
            <Input
              id="import-season"
              type="number"
              min={1}
              max={100}
              value={seasonNumber}
              onChange={(e) => setSeasonNumber(e.target.value)}
              className="w-28 border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="import-json" className="text-zinc-300">owen-pak JSON</Label>
            <Textarea
              id="import-json"
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder='{"battlePassOfferId":"...","paidRewards":[...],"freeRewards":[...]}'
              className="max-h-72 min-h-40 border-zinc-700 bg-zinc-950 font-mono text-xs text-zinc-200 placeholder:text-zinc-600"
              spellCheck={false}
            />
          </div>
          <Button
            onClick={() => void submit()}
            disabled={busy || !text.trim() || !seasonNumber}
            className="bg-emerald-400 text-zinc-950 hover:bg-emerald-300"
          >
            {busy ? <Loader2 className="size-4 animate-spin" /> : null}
            Import season
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Servers section
// ---------------------------------------------------------------------------

const ALL_FEATURES: Array<{ id: string; label: string }> = [
  { id: "profiles", label: "Profiles (MCP)" },
  { id: "matchmaking", label: "Matchmaking" },
  { id: "storefront", label: "Storefront" },
  { id: "cloudstorage", label: "Cloudstorage" },
  { id: "xmpp", label: "XMPP" },
  { id: "lightswitch", label: "Lightswitch" },
  { id: "contentpages", label: "Content Pages" },
  { id: "keychain", label: "Keychain" },
  { id: "manifest", label: "Manifest" },
];

const ENDPOINT_MAP: Array<{ epic: string; route: string }> = [
  { epic: "MCP profile", route: "/fortnite/api/game/v2/profile/:clientId/profile/:accountId" },
  { epic: "Storefront", route: "/fortnite/api/storefront/v2/catalog" },
  { epic: "Cloudstorage", route: "/fortnite/api/cloudstorage/system" },
  { epic: "Lightswitch", route: "/fortnite/api/lightswitch/api/service/bulk/status" },
  { epic: "Assets", route: "/launcher/api/public/assets/:platform/:catalogItemId/:appName" },
  { epic: "Waitingroom", route: "/waitingroom/api/waitingroom" },
  { epic: "Keychain", route: "/fortnite/api/keychain" },
  { epic: "Contentpages", route: "/fortnite/api/contentpages/pageConfig" },
];

type HealthState =
  | { status: "idle" }
  | { status: "checking" }
  | { status: "ok"; httpStatus: number; elapsedMs: number }
  | { status: "fail"; message: string };

function ServersSection() {
  const [servers, setServers] = useState<ServerProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [health, setHealth] = useState<Record<string, HealthState>>({});
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<ServerProfile | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/servers");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setServers(await res.json());
    } catch {
      toast.error("Failed to load servers");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const checkHealth = async (id: string) => {
    setHealth((h) => ({ ...h, [id]: { status: "checking" } }));
    try {
      const res = await fetch("/api/servers/check", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id }),
      });
      const json = await res.json();
      if (json.ok) {
        setHealth((h) => ({
          ...h,
          [id]: { status: "ok", httpStatus: json.status, elapsedMs: json.elapsedMs },
        }));
        toast.success(`Lightswitch OK (HTTP ${json.status}, ${json.elapsedMs} ms)`);
      } else if (res.status === 502) {
        setHealth((h) => ({ ...h, [id]: { status: "fail", message: json.error || "Connection failed" } }));
        toast.error(json.error || "Health check failed");
      } else {
        setHealth((h) => ({
          ...h,
          [id]: { status: "fail", message: `HTTP ${json.status ?? res.status} ${json.statusText ?? ""}`.trim() },
        }));
        toast.error(`Lightswitch responded HTTP ${json.status ?? res.status}`);
      }
    } catch {
      setHealth((h) => ({ ...h, [id]: { status: "fail", message: "Network error" } }));
      toast.error("Health check request failed");
    }
  };

  const remove = async (id: string, name: string) => {
    try {
      const res = await fetch(`/api/servers?id=${encodeURIComponent(id)}`, { method: "DELETE" });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      toast.success(`Server "${name}" deleted`);
      void load();
    } catch {
      toast.error("Failed to delete server");
    }
  };

  return (
    <div>
      <SectionHeading
        title="Servers"
        description="Backend server profiles, feature flags and live health checks."
      />

      <div className="mb-4">
        <Button
          onClick={() => {
            setEditing(null);
            setFormOpen(true);
          }}
          className="bg-emerald-400 text-zinc-950 hover:bg-emerald-300"
        >
          <Plus className="size-4" /> Add server
        </Button>
      </div>

      {loading && servers.length === 0 ? (
        <div className="space-y-3">
          {Array.from({ length: 2 }).map((_, i) => (
            <div key={i} className="h-28 animate-pulse rounded-lg border border-zinc-800 bg-zinc-900" />
          ))}
        </div>
      ) : servers.length === 0 ? (
        <Card className="border-zinc-800 bg-zinc-900">
          <CardContent className="p-6 text-sm text-zinc-500">
            No servers configured yet. Add a profile pointing at your LawinServer v2 or
            Reload-Backend instance.
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {servers.map((s) => {
            const h = health[s.id] ?? { status: "idle" };
            return (
              <Card key={s.id} className="border-zinc-800 bg-zinc-900">
                <CardContent className="p-4 sm:p-6">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="text-sm font-semibold text-zinc-100">{s.name}</p>
                        <Badge className="border-zinc-700 bg-zinc-800 text-[10px] text-zinc-300 hover:bg-zinc-800">
                          {s.backend}
                        </Badge>
                        {h.status === "ok" ? (
                          <Badge className="border-emerald-400/30 bg-emerald-400/10 text-[10px] text-emerald-400 hover:bg-emerald-400/10">
                            OK HTTP {h.httpStatus} ({h.elapsedMs} ms)
                          </Badge>
                        ) : null}
                        {h.status === "fail" ? (
                          <Badge className="border-red-400/30 bg-red-400/10 text-[10px] text-red-400 hover:bg-red-400/10">
                            FAIL — {h.message}
                          </Badge>
                        ) : null}
                      </div>
                      <p className="mt-1 truncate font-mono text-xs text-zinc-400">{s.baseUrl}</p>
                      <div className="mt-2 flex flex-wrap gap-1.5">
                        {s.features.length === 0 ? (
                          <span className="text-xs text-zinc-600">No features enabled</span>
                        ) : (
                          s.features.map((f) => (
                            <span
                              key={f}
                              className="rounded border border-zinc-800 bg-zinc-950 px-1.5 py-0.5 text-[10px] text-zinc-400"
                            >
                              {f}
                            </span>
                          ))
                        )}
                      </div>
                    </div>
                    <div className="flex shrink-0 flex-wrap gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => void checkHealth(s.id)}
                        disabled={h.status === "checking"}
                        className="h-11 border-zinc-700 bg-zinc-950 text-zinc-200 hover:bg-zinc-800"
                      >
                        {h.status === "checking" ? (
                          <Loader2 className="size-4 animate-spin" />
                        ) : (
                          <Activity className="size-4" />
                        )}
                        Check health
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-11 text-zinc-400 hover:text-emerald-400"
                        aria-label={`Edit ${s.name}`}
                        onClick={() => {
                          setEditing(s);
                          setFormOpen(true);
                        }}
                      >
                        <Pencil className="size-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-11 text-zinc-400 hover:text-red-400"
                        aria-label={`Delete ${s.name}`}
                        onClick={() => void remove(s.id, s.name)}
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <Card className="mt-6 border-zinc-800 bg-zinc-900">
        <CardHeader className="pb-3">
          <CardTitle className="text-base text-zinc-100">Endpoint mapping reference</CardTitle>
          <CardDescription className="text-zinc-500">
            Epic endpoint to backend route, as implemented by LawinServer v2 and Reload-Backend.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow className="border-zinc-800 hover:bg-transparent">
                <TableHead className="text-zinc-400">Epic endpoint</TableHead>
                <TableHead className="text-zinc-400">Backend route</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {ENDPOINT_MAP.map((row) => (
                <TableRow key={row.epic} className="border-zinc-800">
                  <TableCell className="text-sm text-zinc-300">{row.epic}</TableCell>
                  <TableCell className="font-mono text-xs text-emerald-400">{row.route}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <ServerFormDialog
        open={formOpen}
        onOpenChange={setFormOpen}
        editing={editing}
        onSaved={() => void load()}
      />
    </div>
  );
}

function ServerFormDialog({
  open,
  onOpenChange,
  editing,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editing: ServerProfile | null;
  onSaved: () => void;
}) {
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [backend, setBackend] = useState("lawin-v2");
  const [features, setFeatures] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open) {
      setName(editing?.name ?? "");
      setBaseUrl(editing?.baseUrl ?? "");
      setBackend(editing?.backend ?? "lawin-v2");
      setFeatures(editing?.features ?? []);
    }
  }, [open, editing]);

  const toggleFeature = (id: string, checked: boolean) => {
    setFeatures((f) => (checked ? [...f, id] : f.filter((x) => x !== id)));
  };

  const submit = async () => {
    if (!name.trim() || !baseUrl.trim()) {
      toast.error("Name and base URL are required");
      return;
    }
    setBusy(true);
    try {
      const res = await fetch("/api/servers", {
        method: editing ? "PATCH" : "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(
          editing ? { id: editing.id, name, baseUrl, backend, features } : { name, baseUrl, backend, features }
        ),
      });
      if (!res.ok) {
        const json = await res.json().catch(() => ({}));
        throw new Error(json?.error || `HTTP ${res.status}`);
      }
      toast.success(editing ? "Server updated" : "Server created");
      onSaved();
      onOpenChange(false);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Save failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85vh] overflow-y-auto border-zinc-800 bg-zinc-900 sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="text-zinc-100">{editing ? "Edit server" : "Add server"}</DialogTitle>
          <DialogDescription className="text-zinc-500">
            Backend profile used by the Redirecter and health checks.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="server-name" className="text-zinc-300">Name</Label>
            <Input
              id="server-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Production reload-backend"
              className="border-zinc-700 bg-zinc-950 text-sm text-zinc-100 placeholder:text-zinc-600"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="server-url" className="text-zinc-300">Base URL</Label>
            <Input
              id="server-url"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="http://192.168.1.10:8000"
              className="border-zinc-700 bg-zinc-950 font-mono text-sm text-zinc-100 placeholder:text-zinc-600"
              autoComplete="off"
              spellCheck={false}
            />
            <p className="text-xs text-zinc-500">HTTP and HTTPS are both accepted.</p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="server-backend" className="text-zinc-300">Backend type</Label>
            <Select value={backend} onValueChange={setBackend}>
              <SelectTrigger id="server-backend" className="border-zinc-700 bg-zinc-950 text-zinc-100">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="border-zinc-700 bg-zinc-900 text-zinc-100">
                <SelectItem value="lawin-v2">lawin-v2</SelectItem>
                <SelectItem value="reload-backend">reload-backend</SelectItem>
                <SelectItem value="custom">custom</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <fieldset className="space-y-2">
            <legend className="text-sm text-zinc-300">Enabled features</legend>
            <div className="grid grid-cols-2 gap-2 rounded-md border border-zinc-800 bg-zinc-950/60 p-3">
              {ALL_FEATURES.map((f) => (
                <label key={f.id} className="flex min-h-11 cursor-pointer items-center gap-2 text-sm text-zinc-300">
                  <Checkbox
                    checked={features.includes(f.id)}
                    onCheckedChange={(v) => toggleFeature(f.id, v === true)}
                    className="border-zinc-600 data-[state=checked]:border-emerald-400 data-[state=checked]:bg-emerald-400 data-[state=checked]:text-zinc-950"
                    aria-label={f.label}
                  />
                  {f.label}
                </label>
              ))}
            </div>
          </fieldset>
          <Button
            onClick={() => void submit()}
            disabled={busy}
            className="bg-emerald-400 text-zinc-950 hover:bg-emerald-300"
          >
            {busy ? <Loader2 className="size-4 animate-spin" /> : null}
            {editing ? "Save changes" : "Create server"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Keys section
// ---------------------------------------------------------------------------

function KeysSection() {
  const [keys, setKeys] = useState<KeyEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [generating, setGenerating] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/keys");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setKeys(await res.json());
    } catch {
      toast.error("Failed to load access keys");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const generate = async () => {
    setGenerating(true);
    try {
      const res = await fetch("/api/keys", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ role: "player" }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      toast.success("New key generated");
      void load();
    } catch {
      toast.error("Failed to generate key");
    } finally {
      setGenerating(false);
    }
  };

  const update = async (id: string, patch: Record<string, unknown>) => {
    try {
      const res = await fetch("/api/keys", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, ...patch }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setKeys((ks) => ks.map((k) => (k.id === id ? { ...k, ...(patch as Partial<KeyEntry>) } : k)));
      toast.success("Key updated");
    } catch {
      toast.error("Failed to update key");
    }
  };

  const remove = async (id: string, key: string) => {
    try {
      const res = await fetch(`/api/keys?id=${encodeURIComponent(id)}`, { method: "DELETE" });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      toast.success(`Key ${key} deleted`);
      void load();
    } catch {
      toast.error("Failed to delete key");
    }
  };

  const q = query.trim().toLowerCase();
  const filtered = q
    ? keys.filter(
        (k) =>
          k.key.toLowerCase().includes(q) ||
          k.note.toLowerCase().includes(q) ||
          k.role.toLowerCase().includes(q)
      )
    : keys;

  return (
    <div>
      <SectionHeading
        title="Keys"
        description="Access key management (this replaces the classic Discord bot admin panel)."
      />

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <div className="relative min-w-56 flex-1">
          <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-zinc-500" aria-hidden="true" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search by key, note or role"
            aria-label="Search access keys"
            className="h-11 border-zinc-700 bg-zinc-950 pl-9 text-sm text-zinc-100 placeholder:text-zinc-600"
          />
        </div>
        <Button
          onClick={() => void generate()}
          disabled={generating}
          className="h-11 bg-emerald-400 text-zinc-950 hover:bg-emerald-300"
        >
          {generating ? <Loader2 className="size-4 animate-spin" /> : <Plus className="size-4" />}
          Generate key
        </Button>
      </div>

      <Card className="border-zinc-800 bg-zinc-900">
        <CardContent className="p-0">
          {loading && keys.length === 0 ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="size-6 animate-spin text-emerald-400" aria-hidden="true" />
            </div>
          ) : filtered.length === 0 ? (
            <p className="p-6 text-sm text-zinc-500">
              {q ? "No keys match the search." : "No access keys yet. Generate the first one."}
            </p>
          ) : (
            <div className={filtered.length > 8 ? "max-h-96 overflow-y-auto" : ""}>
              <Table>
                <TableHeader className="sticky top-0 z-10 bg-zinc-900">
                  <TableRow className="border-zinc-800 hover:bg-transparent">
                    <TableHead className="text-zinc-400">Key</TableHead>
                    <TableHead className="text-zinc-400">Role</TableHead>
                    <TableHead className="text-zinc-400">Note</TableHead>
                    <TableHead className="text-zinc-400">Created</TableHead>
                    <TableHead className="text-zinc-400">Last used</TableHead>
                    <TableHead className="text-zinc-400">Revoked</TableHead>
                    <TableHead className="sr-only">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filtered.map((k) => (
                    <TableRow key={k.id} className="border-zinc-800">
                      <TableCell className="font-mono text-xs text-zinc-200">
                        <span className={`inline-flex items-center gap-1 ${k.revoked ? "line-through opacity-50" : ""}`}>
                          {k.key}
                          <CopyButton value={k.key} label="Key" />
                        </span>
                      </TableCell>
                      <TableCell>
                        <Select
                          value={k.role}
                          onValueChange={(v) => void update(k.id, { role: v })}
                        >
                          <SelectTrigger
                            className="h-9 w-32 border-zinc-700 bg-zinc-950 text-xs text-zinc-200"
                            aria-label={`Role for key ${k.key}`}
                          >
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent className="border-zinc-700 bg-zinc-900 text-zinc-100">
                            <SelectItem value="admin">admin</SelectItem>
                            <SelectItem value="moderator">moderator</SelectItem>
                            <SelectItem value="player">player</SelectItem>
                          </SelectContent>
                        </Select>
                      </TableCell>
                      <TableCell>
                        <Input
                          defaultValue={k.note}
                          placeholder="add note"
                          aria-label={`Note for key ${k.key}`}
                          className="h-9 w-36 border-zinc-700 bg-zinc-950 text-xs text-zinc-200 placeholder:text-zinc-600"
                          onBlur={(e) => {
                            if (e.target.value !== k.note) void update(k.id, { note: e.target.value });
                          }}
                        />
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-xs text-zinc-400">
                        {formatDate(k.createdAt)}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-xs text-zinc-400">
                        {formatDate(k.lastUsedAt)}
                      </TableCell>
                      <TableCell>
                        <Switch
                          checked={k.revoked}
                          onCheckedChange={(v) => void update(k.id, { revoked: v })}
                          className="data-[state=checked]:bg-red-400"
                          aria-label={`Revoke key ${k.key}`}
                        />
                      </TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-9 text-zinc-500 hover:text-red-400"
                          aria-label={`Delete key ${k.key}`}
                          onClick={() => void remove(k.id, k.key)}
                        >
                          <Trash2 className="size-4" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Documentation section
// ---------------------------------------------------------------------------

const DOCS_SECTIONS: Array<{ id: string; title: string; content: React.ReactNode }> = [
  {
    id: "overview",
    title: "Overview",
    content: (
      <div className="space-y-3 text-sm leading-relaxed text-zinc-400">
        <p>
          Owen Launcher RS is a Fortnite private-server launcher toolchain written in Rust.
          A single library, <span className="font-mono text-emerald-400">libowen.so</span>, is
          injected into the game APK by <span className="font-mono">owen-apk</span> and loaded at
          startup. It reads <span className="font-mono">owen.json</span> from the device, then
          redirects every Epic endpoint to your backend.
        </p>
        <p>
          The defining property is that it requires no smali edits whatsoever: the old Owen.c
          needed a per-version hardcoded offset table (18.40+ only), and older stacks patched
          const-strings inside compiled Kotlin. Owen instead locates{" "}
          <span className="font-mono">curl_easy_setopt</span> at runtime with a structural
          signature scan, covering versions 10.31 through 33.00+ with the same binary.
        </p>
      </div>
    ),
  },
  {
    id: "no-smali",
    title: "The no-smali redirect stack",
    content: (
      <ol className="space-y-3 text-sm leading-relaxed text-zinc-400">
        <li>
          <span className="font-medium text-zinc-200">Layer 1 — curl hook.</span> All engine HTTP
          traffic flows through curl_easy_setopt in libUE4.so / libUnreal.so / libEOSSDK.so.
          Hooking it redirects every request to the backend.
        </li>
        <li>
          <span className="font-medium text-zinc-200">Layer 2 — ART hooks.</span>{" "}
          WebView.loadUrl, DebouncedLimiter.loginUrl(Environment,boolean) and
          getGraphqlUrl(Environment) are hooked in the ART runtime, covering the login page and
          the Kairos PartyHub on versions 10.31 through 15.50.
        </li>
        <li>
          <span className="font-medium text-zinc-200">Layer 3 — cleartext unlock.</span>{" "}
          NetworkSecurityPolicy.isCleartextTrafficPermitted is forced to true so plain-HTTP
          login pages work.
        </li>
        <li>
          <span className="font-medium text-zinc-200">Layer 4 — auto manifest redirect.</span>{" "}
          Manifest and asset CDN URLs are rewritten to the backend automatically when manifest
          mode is auto.
        </li>
      </ol>
    ),
  },
  {
    id: "setup",
    title: "Setup guide",
    content: (
      <ol className="space-y-3 text-sm leading-relaxed text-zinc-400">
        <li>
          <span className="font-medium text-zinc-200">1. Build libowen.so.</span> Use the GitHub
          Actions workflow targeting aarch64-linux-android. The workflow produces the release
          library artifact.
        </li>
        <li>
          <span className="font-medium text-zinc-200">2. Put owen.json on the device.</span>{" "}
          Export it from the Redirecter tab and push it to{" "}
          <span className="font-mono text-xs text-emerald-400">
            /sdcard/Android/data/&lt;game&gt;/files/owen.json
          </span>{" "}
          or{" "}
          <span className="font-mono text-xs text-emerald-400">/data/local/tmp/owen.json</span>.
        </li>
        <li>
          <span className="font-medium text-zinc-200">3. Patch the APK with owen-apk.</span>{" "}
          owen-apk injects the library into the APK and the game loads libowen.so which
          auto-detects curl_easy_setopt on ANY version.
        </li>
        <li>
          <span className="font-medium text-zinc-200">4. Ship it.</span> Install the patched APK
          on the device; no smali edits, no offset tables, no per-version builds.
        </li>
      </ol>
    ),
  },
  {
    id: "http-https",
    title: "HTTP and HTTPS",
    content: (
      <div className="space-y-3 text-sm leading-relaxed text-zinc-400">
        <p>
          Both schemes are accepted verbatim in the server base URL. The Redirecter never
          rewrites or auto-prepends the scheme: if you type{" "}
          <span className="font-mono text-emerald-400">http://</span> you get HTTP, if you type{" "}
          <span className="font-mono text-emerald-400">https://</span> you get HTTPS.
        </p>
        <p>
          For plain-HTTP login pages, Android would normally block cleartext traffic through the
          NetworkSecurityPolicy. The webview option &quot;Force cleartext HTTP&quot; (default
          on) forces isCleartextTrafficPermitted to true at runtime, so HTTP login pages work
          without repackaging the APK with a custom policy file.
        </p>
      </div>
    ),
  },
  {
    id: "methodology",
    title: "Symbol Finder methodology",
    content: (
      <div className="space-y-3 text-sm leading-relaxed text-zinc-400">
        <p>
          The four layers: (1) <span className="font-mono text-emerald-400">MOV W0, #0x2b</span>{" "}
          — CURLE_BAD_FUNCTION_ARGUMENT (43), returned when the handle is NULL; (2) a{" "}
          <span className="font-mono text-emerald-400">CBZ X0</span> branching to that MOV; (3) a{" "}
          <span className="font-mono text-emerald-400">BL</span> to Curl_vsetopt between them;
          (4) a backward walk to the function prologue.
        </p>
        <p>
          v2 improvements: closest-marker backward walk, widened stp Xt1,Xt2,[sp,#-N]! markers,
          predecessor sanity check, and the variadic +3 fingerprint bonus (two or more STP Q
          pairs in the prologue is the va_start spill).
        </p>
      </div>
    ),
  },
  {
    id: "battlepass-workflow",
    title: "Battle Pass extraction workflow",
    content: (
      <div className="space-y-3 text-sm leading-relaxed text-zinc-400">
        <p>
          <span className="font-mono text-emerald-400">owen-pak</span> reads the game .pak
          files, locates AthenaSeasonItemDefinition assets per season and extracts the reward
          tables (paid and free) together with the offer IDs.
        </p>
        <p>
          The extracted JSON is imported here via the Battle Pass tab (Import dialog), stored in
          the console database, and can be exported again in the exact Reload-Backend shape for
          your server configuration.
        </p>
        <p>
          <span className="font-medium text-zinc-200">Seasons 21 to 42 are generated.</span>{" "}
          Reload-Backend ships real tables up to Season 20; this console builds 21-42 with a
          data-driven pipeline: the season launch cluster from the fortnite-api cosmetics dump
          (every item whose added date equals the historical launch date — validated to contain
          100 percent of the real Season 20 cosmetics), families ordered by Epic sequential
          token ids, and matching catalog storefront entries (pass 950 / bundle 2800 / tier 150
          V-Bucks) available from the Catalog 21+ button.
        </p>
        <p>
          Tier placement follows family page order; free tracks carry the standard V-Bucks and
          small-item distribution. When a player buys the pass, the backend grants rewards
          tier-by-tier up to their level exactly like Reload-Backend does.
        </p>
      </div>
    ),
  },
  {
    id: "backends",
    title: "Backend compatibility",
    content: (
      <div className="space-y-3 text-sm leading-relaxed text-zinc-400">
        <p>
          <span className="font-medium text-zinc-200">LawinServer v2</span> — supported. MCP
          profiles, storefront, cloudstorage, lightswitch, assets, waitingroom, keychain and
          content pages endpoints are mapped and health-checked.
        </p>
        <p>
          <span className="font-medium text-zinc-200">Reload-Backend</span> — supported. Season
          data exports use its exact battle pass shape, and the same endpoint surface is
          verified with the per-server health check.
        </p>
        <p>
          See the Servers tab for the full endpoint mapping reference table.
        </p>
      </div>
    ),
  },
  {
    id: "faq",
    title: "FAQ",
    content: (
      <div className="space-y-4 text-sm leading-relaxed text-zinc-400">
        <div>
          <p className="font-medium text-zinc-200">Does the scanner need offset tables?</p>
          <p className="mt-1">No. The signature scan works on any stripped ARM64 build; tables were the old approach.</p>
        </div>
        <div>
          <p className="font-medium text-zinc-200">Which versions are supported?</p>
          <p className="mt-1">10.31 through 33.00+ with the same libowen.so binary, plus libEOSSDK.so coverage from 22.00.</p>
        </div>
        <div>
          <p className="font-medium text-zinc-200">Can I run the backend over plain HTTP?</p>
          <p className="mt-1">Yes. Enter an http:// base URL and keep Force cleartext HTTP enabled.</p>
        </div>
        <div>
          <p className="font-medium text-zinc-200">Where does owen.json live on the device?</p>
          <p className="mt-1">/sdcard/Android/data/&lt;game&gt;/files/owen.json or /data/local/tmp/owen.json.</p>
        </div>
        <div>
          <p className="font-medium text-zinc-200">Is PartyHub patched via smali?</p>
          <p className="mt-1">Never. ART runtime hooks replace all smali const-string patching.</p>
        </div>
      </div>
    ),
  },
];

function DocsSection() {
  return (
    <div>
      <SectionHeading
        title="Documentation"
        description="Everything about the Owen Launcher RS toolchain, in one page."
      />
      <Card className="border-zinc-800 bg-zinc-900">
        <CardContent className="p-2 sm:p-4">
          <Accordion type="single" collapsible defaultValue="overview">
            {DOCS_SECTIONS.map((s) => (
              <AccordionItem key={s.id} value={s.id} className="border-zinc-800">
                <AccordionTrigger className="text-sm text-zinc-200 hover:text-emerald-400 hover:no-underline">
                  {s.title}
                </AccordionTrigger>
                <AccordionContent className="pb-4">{s.content}</AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Root console
// ---------------------------------------------------------------------------

export default function OwenConsole() {
  const [tab, setTab] = useState<TabId>("dashboard");
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const [status, setStatus] = useState<StatusPayload | null>(null);
  const [statusLoading, setStatusLoading] = useState(true);

  const refreshStatus = useCallback(async () => {
    setStatusLoading(true);
    try {
      const res = await fetch("/api/status");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setStatus(await res.json());
    } catch {
      // Dashboard keeps last known state; errors surface per-card.
    } finally {
      setStatusLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  const navigate = (t: TabId) => {
    setTab(t);
    setMobileNavOpen(false);
  };

  const navButtons = (compact: boolean) => (
    <>
      {NAV_ITEMS.map((item) => {
        const Icon = item.icon;
        const active = tab === item.id;
        return (
          <button
            key={item.id}
            role="tab"
            aria-selected={active}
            aria-controls={`panel-${item.id}`}
            tabIndex={active ? 0 : -1}
            onClick={() => navigate(item.id)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown" || e.key === "ArrowRight") {
                e.preventDefault();
                const idx = NAV_ITEMS.findIndex((n) => n.id === item.id);
                const next = NAV_ITEMS[(idx + 1) % NAV_ITEMS.length];
                navigate(next.id);
              } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
                e.preventDefault();
                const idx = NAV_ITEMS.findIndex((n) => n.id === item.id);
                const prev = NAV_ITEMS[(idx - 1 + NAV_ITEMS.length) % NAV_ITEMS.length];
                navigate(prev.id);
              }
            }}
            className={`flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2.5 text-sm outline-none transition-colors focus-visible:ring-2 focus-visible:ring-emerald-400/50 ${
              active
                ? "bg-emerald-400/10 font-medium text-emerald-400"
                : "text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-200"
            } ${compact ? "w-full" : ""}`}
          >
            <Icon className="size-4 shrink-0" aria-hidden="true" />
            {item.label}
          </button>
        );
      })}
    </>
  );

  return (
    <div className="flex min-h-screen flex-col bg-zinc-950 font-sans text-zinc-100">
      <header className="sticky top-0 z-40 border-b border-zinc-800 bg-zinc-950/95 backdrop-blur supports-[backdrop-filter]:bg-zinc-950/80">
        <div className="flex h-14 items-center gap-3 px-4 sm:px-6">
          <Sheet open={mobileNavOpen} onOpenChange={setMobileNavOpen}>
            <SheetTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="size-11 text-zinc-400 hover:text-zinc-100 lg:hidden"
                aria-label="Open navigation menu"
              >
                <Menu className="size-5" />
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-64 border-r border-zinc-800 bg-zinc-950 p-3">
              <SheetTitle className="px-2 pb-2 pt-1 text-sm font-semibold text-zinc-100">
                <span className="flex items-center gap-2">
                  <OwenMark className="size-6 text-emerald-400" />
                  Owen Launcher
                </span>
              </SheetTitle>
              <nav aria-label="Console sections (mobile)" className="flex flex-col gap-1">
                {navButtons(true)}
              </nav>
            </SheetContent>
          </Sheet>

          <div className="flex items-center gap-2.5">
            <OwenMark className="size-8 text-emerald-400" />
            <div className="leading-tight">
              <p className="text-sm font-semibold tracking-tight">Owen Launcher</p>
              <p className="hidden text-[11px] text-zinc-500 sm:block">Web Console</p>
            </div>
          </div>

          <div className="ml-auto flex items-center gap-2">
            <Badge
              variant="outline"
              className="hidden border-emerald-400/30 bg-emerald-400/10 text-[10px] font-medium text-emerald-400 hover:bg-emerald-400/10 sm:inline-flex"
            >
              <Network className="mr-1 size-3" aria-hidden="true" />
              no-smali stack
            </Badge>
            <Button
              variant="ghost"
              size="icon"
              className="size-11 text-zinc-400 hover:text-zinc-100"
              aria-label="Refresh console status"
              onClick={() => void refreshStatus()}
            >
              <RefreshCw className={`size-4 ${statusLoading ? "animate-spin" : ""}`} />
            </Button>
          </div>
        </div>
      </header>

      <div className="flex flex-1">
        <nav
          aria-label="Console sections"
          className="sticky top-14 hidden h-[calc(100vh-3.5rem)] w-60 shrink-0 flex-col gap-1 overflow-y-auto border-r border-zinc-800 bg-zinc-950 p-3 lg:flex"
        >
          {navButtons(false)}
          <div className="mt-auto rounded-md border border-zinc-800 bg-zinc-900/60 p-3">
            <p className="flex items-center gap-1.5 text-[11px] font-medium text-zinc-400">
              <Gauge className="size-3.5 text-emerald-400" aria-hidden="true" />
              Console status
            </p>
            <p className="mt-1 text-[11px] leading-relaxed text-zinc-500">
              {status?.redirecter.configured ? "Redirecter configured." : "Redirecter not configured."}
              {status ? ` ${status.keys} keys, ${status.servers} servers.` : ""}
            </p>
          </div>
        </nav>

        <main id="main-content" className="min-w-0 flex-1 p-4 sm:p-6">
          <div
            role="tabpanel"
            id={`panel-${tab}`}
            aria-labelledby={`tab-${tab}`}
            className="mx-auto w-full max-w-6xl"
          >
            {tab === "dashboard" ? (
              <DashboardSection
                status={status}
                loading={statusLoading}
                onRefresh={() => void refreshStatus()}
                onNavigate={navigate}
              />
            ) : null}
            {tab === "redirecter" ? <RedirecterSection /> : null}
            {tab === "finder" ? <SymbolFinderSection /> : null}
            {tab === "partyhub" ? <PartyHubSection /> : null}
            {tab === "battlepass" ? <BattlePassSection /> : null}
            {tab === "servers" ? <ServersSection /> : null}
            {tab === "keys" ? <KeysSection /> : null}
            {tab === "docs" ? <DocsSection /> : null}
          </div>
        </main>
      </div>

      <footer className="mt-auto border-t border-zinc-800 bg-zinc-950 pb-[env(safe-area-inset-bottom)]">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-2 px-4 py-4 text-xs text-zinc-500 sm:px-6">
          <p>
            Owen Launcher Web Console — private-server toolchain. Not affiliated with Epic Games.
          </p>
          <p className="flex items-center gap-1.5">
            <ShieldCheck className="size-3.5 text-emerald-400" aria-hidden="true" />
            no-smali redirect stack
          </p>
        </div>
      </footer>
    </div>
  );
}
