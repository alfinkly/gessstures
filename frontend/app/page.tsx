'use client'

import { useEffect, useRef, useCallback } from 'react'
import dynamic from 'next/dynamic'
import { useWs } from '@/lib/ws'

const GraphCanvas = dynamic(() => import('@/components/GraphCanvas'), { ssr: false })

export default function GraphPage() {
  const snapshotRef = useRef<{ nodes: number; edges: number }>({ nodes: 0, edges: 0 })
  const fpsRef = useRef(0)
  const frameCount = useRef(0)
  const lastFpsTime = useRef(performance.now())

  const onSnapshot = useCallback((snap: any) => {
    snapshotRef.current = { nodes: snap.nodes.length, edges: snap.edges.length }
    ;(window as any).__updateGraph?.(snap)
    frameCount.current++
    const now = performance.now()
    if (now - lastFpsTime.current >= 1000) {
      fpsRef.current = frameCount.current
      frameCount.current = 0
      lastFpsTime.current = now
    }
  }, [])

  const { send } = useWs(onSnapshot, useCallback(() => {}, []))

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'p') send({ t: 'pause' })
      if (e.key === 'r') send({ t: 'resume' })
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [send])

  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative' }}>
      <GraphCanvas />
      <div style={{
        position: 'absolute', top: 12, left: 12,
        display: 'flex', gap: 16, fontSize: 13,
        color: 'rgba(255,255,255,0.7)',
        background: 'rgba(0,0,0,0.6)', padding: '6px 14px',
        borderRadius: 8, backdropFilter: 'blur(8px)', pointerEvents: 'none',
      }}>
        <span id="node-count">{snapshotRef.current.nodes} nodes</span>
        <span id="fps">{fpsRef.current} fps</span>
      </div>
      <div style={{
        position: 'absolute', bottom: 16, left: '50%', transform: 'translateX(-50%)',
        fontSize: 12, color: 'rgba(255,255,255,0.4)', pointerEvents: 'none',
      }}>
        Drag: orbit &bull; Scroll: zoom &bull; Right-drag: pan &bull;
        P: pause physics &bull; R: resume
        <br />
        <a href="/body-detect" style={{ color: '#6366f1' }}>Body Detect Test</a>
      </div>
    </div>
  )
}
