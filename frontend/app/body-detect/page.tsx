'use client'

import dynamic from 'next/dynamic'
import { useWs } from '@/lib/ws'

const BodyDetectView = dynamic(() => import('@/components/BodyDetectView'), { ssr: false })

export default function BodyDetectPage() {
  const { sendBinary } = useWs(
    () => {},
    (result) => { (window as any).__bodyResult?.(result) },
  )

  return <BodyDetectView sendBinary={sendBinary} />
}
