import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  /* config options here */
  typescript: {
    ignoreBuildErrors: true,
  },
  reactStrictMode: false,
  async redirects() {
    return [
      /* convenience: /partyhub -> the PartyHub web entry */
      { source: "/partyhub", destination: "/partyhub/offline/en-US/", permanent: false },
    ];
  },
  async rewrites() {
    return [
      /* the patched Kairos PartyHub web (served by mini-services/partyhub-web) */
      { source: "/partyhub/:path*", destination: "http://127.0.0.1:8091/:path*" },
    ];
  },
};

export default nextConfig;
