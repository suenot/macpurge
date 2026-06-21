import type { Metadata } from 'next';
import './globals.css';
import { Sidebar } from '@/components/layout/Sidebar';
import Onboarding from '@/components/Onboarding';

export const metadata: Metadata = {
  title: 'MacPurge',
  description: 'Disk cleanup for developers',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen bg-white dark:bg-dark-bg text-slate-900 dark:text-white bg-gradient-glow selection:bg-brand-500/30">
        <div className="flex h-screen">
          <Sidebar />
          <main className="flex-1 overflow-auto p-6">
            {children}
          </main>
        </div>
        <Onboarding />
      </body>
    </html>
  );
}
