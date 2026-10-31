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
  title: "غرفة تحكم أوين — Owen Control Room",
  description:
    "Owen Control Room: human-in-the-loop Android emulator control — live phone stream, button annotation, manual signals and full GitHub Actions run management for the Owen Launcher / OGS-12.41 pipeline.",
  keywords: [
    "Owen Launcher",
    "Owen Control Room",
    "Fortnite",
    "private server",
    "OGS-12.41",
    "emulator",
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
