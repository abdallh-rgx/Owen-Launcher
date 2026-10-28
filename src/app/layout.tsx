import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Toaster } from "@/components/ui/sonner";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Owen Launcher Web Console",
  description:
    "Professional console for the Owen Launcher Fortnite private-server toolchain: Redirecter configuration, curl_easy_setopt Symbol Finder, Battle Pass seasons, servers and access keys.",
  keywords: [
    "Owen Launcher",
    "Fortnite",
    "private server",
    "curl_easy_setopt",
    "ARM64",
    "no-smali",
  ],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`${geistSans.variable} ${geistMono.variable} bg-zinc-950 font-sans antialiased text-foreground`}
      >
        {children}
        <Toaster position="bottom-right" />
      </body>
    </html>
  );
}
