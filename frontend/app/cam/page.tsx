'use client'

import { useEffect, useRef, useState, useCallback } from 'react'
import { useWs } from '@/lib/ws'

export default function PhoneCamPage() {
  const videoRef = useRef<HTMLVideoElement>(null)
  const capCanvasRef = useRef<HTMLCanvasElement | null>(null)
  const [connected, setConnected] = useState(false)

  useEffect(() => {
    let stream: MediaStream | null = null
    navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false })
      .then(s => { stream = s; if (videoRef.current) { videoRef.current.srcObject = s } })
      .catch(() => {})
    return () => { stream?.getTracks().forEach(t => t.stop()) }
  }, [])

  useEffect(() => {
    const c = document.createElement('canvas')
    c.width = 320; c.height = 240
    capCanvasRef.current = c
  }, [])

  useEffect(() => {
    const w = window as any
    w.__camConnected = setConnected
    return () => { delete (window as any).__camConnected }
  }, [])

  const { sendBinary } = useWs(
    useCallback(() => {}, []),
    useCallback(() => {}, []),
  )

  useEffect(() => {
    const int = setInterval(() => {
      const v = videoRef.current
      const cap = capCanvasRef.current
      if (!v || !cap) return
      const ctx = cap.getContext('2d')
      if (!ctx) return
      ctx.drawImage(v, 0, 0, 320, 240)
      cap.toBlob(blob => {
        if (!blob) return
        blob.arrayBuffer().then(buf => sendBinary(buf))
      }, 'image/jpeg', 0.7)
    }, 300)
    return () => clearInterval(int)
  }, [sendBinary])

  return (
    <div style={{ padding: 12, background: '#0a0a0f', minHeight: '100vh', color: '#e2e8f0' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h2 style={{ fontSize: 18, fontWeight: 600 }}>📷 Camera Stream</h2>
        <span style={{ fontSize: 13, color: connected ? '#4ade80' : '#f87171' }}>
          {connected ? '● online' : '● offline'}
        </span>
      </div>
      <video ref={videoRef} autoPlay playsInline muted style={{ width: '100%', borderRadius: 12, background: '#111' }} />
      <p style={{ fontSize: 12, color: '#64748b', marginTop: 8, textAlign: 'center' }}>
        Sending frames to server…
      </p>
    </div>
  )
}
