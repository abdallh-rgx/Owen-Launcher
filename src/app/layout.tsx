import type { Metadata, Viewport } from "next";
import { Rubik } from "next/font/google";
import "./globals.css";
import { Toaster } from "@/components/ui/sonner";

const rubik = Rubik({
  variable: "--font-rubik",
  subsets: ["arabic", "latin"],
  weight: ["300", "400", "500", "600", "700", "800", "900"],
});

export const metadata: Metadata = {
  title: "Owen Launcher — Liquid Glass",
  description:
    "Owen Launcher: the complete safe-Rust Fortnite mobile private-server stack — launcher, redirecter, LawinServer v2-compatible backend and the native PartyHub, behind a liquid-glass interface.",
  keywords: [
    "Owen Launcher",
    "Fortnite",
    "private server",
    "PartyHub",
    "LawinServer",
    "Reload Backend",
    "Rust",
  ],
};

export const viewport: Viewport = {
  themeColor: "#05060b",
  width: "device-width",
  initialScale: 1,
  viewportFit: "cover",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="ar" dir="rtl" suppressHydrationWarning>
      <body
        className={`${rubik.variable} bg-[#05060b] font-sans antialiased text-[#f4f6fb]`}
      >
        {children}
        <Toaster position="bottom-center" richColors />
      </body>
    </html>
  );
}
