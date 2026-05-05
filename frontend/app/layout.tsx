import type { Metadata } from 'next'
import './globals.css'

export const metadata: Metadata = {
  title: 'Gessstures',
  description: '3D Knowledge Graph with gesture control',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>{children}</body>
    </html>
  )
}
