import type { NextConfig } from "next";

const OWEN_BACKEND = "http://127.0.0.1:3010";

const nextConfig: NextConfig = {
  output: "standalone",
  /* config options here */
  typescript: {
    ignoreBuildErrors: true,
  },
  reactStrictMode: false,
  async redirects() {
    return [
      /* convenience: /partyhub -> the PartyHub web entry (served by the
       * Owen Rust backend) */
      { source: "/partyhub", destination: "/partyhub/offline/en-US/", permanent: false },
    ];
  },
  async rewrites() {
    return [
      /* the Owen Rust backend (owen-launcher-rs/crates/owen-server) — same-origin
       * proxying keeps the Epic-compatible absolute paths intact */
      { source: "/partyhub/:path*", destination: `${OWEN_BACKEND}/partyhub/:path*` },
      { source: "/owen-api/:path*", destination: `${OWEN_BACKEND}/:path*` },
    ];
  },
};

export default nextConfig;
