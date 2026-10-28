// Kairos offline-entry reference server.
// Serves a (patched) shippedweb copy with the three rules from PATCH-GUIDE.md:
//   1. static files + same-origin /graphql endpoint
//   2. SPA fallback (routes look like /offline/en-US/...)
//   3. relative asset resolution from nested paths (./main.kairos...js)
// Logs every request so backend hits are provable.
import { serve } from "bun";
import path from "node:path";
import { existsSync, readFileSync } from "node:fs";

// Folder that contains offline/entry/index.html (the patched copy).
const ROOT = process.env.OWEN_WEB_ROOT ?? "/home/z/my-project/download/shippedweb_owen";
const ENTRY = path.join(ROOT, "offline", "entry");
const PORT = Number(process.env.PORT ?? 8090);

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".map": "application/json",
};

const server = serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    let p = decodeURIComponent(url.pathname);
    const method = req.method;
    const body = method === "POST" ? await req.text() : undefined;
    console.log(`[${new Date().toISOString()}] ${method} ${p}${body ? ` :: ${body.slice(0, 300)}` : ""}`);

    // ---- GraphQL endpoint (same origin) ----
    if (method === "POST" && (p === "/graphql" || p === "/api/graphql")) {
      const q = body ?? "";
      let resp: unknown;
      if (q.includes("myAccount")) {
        resp = {
          data: {
            Account: {
              myAccount: {
                id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                displayName: "OwenTester",
                email: "test@local",
                emailVerified: true,
              },
            },
          },
        };
      } else {
        // Generic GraphQL passthrough shape - replace with your backend proxy.
        resp = { data: {} };
      }
      return new Response(JSON.stringify(resp), {
        headers: {
          "content-type": "application/json",
          "access-control-allow-origin": "*",
        },
      });
    }
    if (method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "access-control-allow-origin": "*",
          "access-control-allow-methods": "GET, POST, OPTIONS",
          "access-control-allow-headers": "*",
        },
      });
    }

    // ---- static files ----
    if (p === "/" || p === "/entry" || p === "/entry/") p = "/offline/entry/index.html";
    const candidates = [
      path.join(ROOT, p.replace(/\.\./g, "")),
      // relative assets from nested SPA paths (e.g. /offline/en-US/main.js -> entry/main.js)
      path.join(ENTRY, path.basename(p)),
    ];
    const file = candidates.find((f) => existsSync(f) && !f.endsWith("/"));
    if (file) {
      const ext = path.extname(file).toLowerCase();
      return new Response(readFileSync(file), {
        headers: { "content-type": MIME[ext] ?? "application/octet-stream" },
      });
    }

    // SPA fallback for routes like /offline/en-US/ and /offline/en-US/component/friends
    return new Response(readFileSync(path.join(ENTRY, "index.html")), {
      headers: { "content-type": "text/html; charset=utf-8" },
    });
  },
});
console.log(`kairos reference server on :${server.port} root=${ROOT}`);
console.log(`open http://localhost:${server.port}/offline/en-US/`);
