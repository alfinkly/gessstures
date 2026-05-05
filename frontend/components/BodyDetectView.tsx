'use client'

import { useEffect, useRef, useState, useCallback } from 'react'
import type { BodyResult, WsConnection } from '@/lib/ws'
import { detectGesture, gestureInfo, shouldShowWaiting, type Gesture } from '@/lib/gestures'

const W = 640, H = 480
const COLORS = ['#6366f1', '#f59e0b', '#10b981', '#ef4444']

const POSE_CONNECTIONS: [number, number][] = [
  [0,1],[1,2],[2,3],[3,7],[0,4],[4,5],[5,6],[6,8],[9,10],
  [11,13],[13,15],[12,14],[14,16],
  [15,17],[15,19],[15,21],[17,19],
  [16,18],[16,20],[16,22],[18,20],
  [11,12],[11,23],[12,24],[23,24],
  [23,25],[25,27],[27,29],[29,31],
  [24,26],[26,28],[28,30],[30,32],
]

function drawSkeleton(ctx: CanvasRenderingContext2D, kp: number[][], w: number, h: number, color: string) {
  for (const [a, b] of POSE_CONNECTIONS) {
    ctx.beginPath()
    ctx.moveTo(kp[a][0] * w, kp[a][1] * h)
    ctx.lineTo(kp[b][0] * w, kp[b][1] * h)
    ctx.strokeStyle = color
    ctx.lineWidth = 2
    ctx.stroke()
  }
  for (const p of kp) {
    ctx.beginPath()
    ctx.arc(p[0] * w, p[1] * h, 3, 0, Math.PI * 2)
    ctx.fillStyle = color
    ctx.fill()
  }
}

type Tile = { wrap: HTMLDivElement; canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D }

export default function BodyDetectView({ sendBinary }: { sendBinary: WsConnection['sendBinary'] }) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const overlayRef = useRef<HTMLCanvasElement>(null)
  const tilesRowRef = useRef<HTMLDivElement>(null)
  const resultRef = useRef<BodyResult | null>(null)
  const tilesRef = useRef<Tile[]>([])
  const captureCanvasRef = useRef<HTMLCanvasElement | null>(null)
  const [connected, setConnected] = useState(false)
  const [personCount, setPersonCount] = useState(0)
  const animRef = useRef<number>(0)
  const intervalRef = useRef<ReturnType<typeof setInterval>>(undefined)

  // per-person gesture tracking
  const rWristHistRef = useRef<Map<number, { x: number; y: number }[]>>(new Map())
  const lWristHistRef = useRef<Map<number, { x: number; y: number }[]>>(new Map())
  const lastActiveRef = useRef<Map<number, number>>(new Map())
  const gestureLabelsRef = useRef<Map<number, HTMLDivElement>>(new Map())

  useEffect(() => {
    const ov = overlayRef.current
    if (!ov) return
    ov.width = W; ov.height = H
  }, [])

  useEffect(() => {
    let stream: MediaStream | null = null
    async function start() {
      stream = await navigator.mediaDevices.getUserMedia({
        video: { width: { exact: W }, height: { exact: H }, facingMode: 'user' },
        audio: false,
      })
      if (videoRef.current) {
        videoRef.current.srcObject = stream
        await videoRef.current.play()
      }
    }
    start().catch(() => {})
    return () => { stream?.getTracks().forEach(t => t.stop()) }
  }, [])

  useEffect(() => {
    const c = document.createElement('canvas')
    c.width = W; c.height = H
    captureCanvasRef.current = c
  }, [])

  useEffect(() => {
    const w = window as any
    w.__bodyResult = (r: BodyResult) => {
      resultRef.current = r
      setPersonCount(r.person_count)
    }
    return () => { delete (window as any).__bodyResult }
  }, [])

  useEffect(() => {
    const w = window as any
    w.__wsConnected = setConnected
    if (w.__wsConn) setConnected(true)
    return () => {}
  }, [])

  useEffect(() => {
    intervalRef.current = setInterval(() => {
      const video = videoRef.current
      const cap = captureCanvasRef.current
      if (!video || !cap) return
      const ctx = cap.getContext('2d')
      if (!ctx) return
      ctx.drawImage(video, 0, 0, W, H)
      cap.toBlob((blob) => {
        if (!blob) return
        blob.arrayBuffer().then(buf => sendBinary(buf))
      }, 'image/jpeg', 0.8)
    }, 120)
    return () => clearInterval(intervalRef.current)
  }, [sendBinary])

  useEffect(() => {
    function render() {
      animRef.current = requestAnimationFrame(render)
      const ov = overlayRef.current
      const video = videoRef.current
      const tilesRow = tilesRowRef.current
      if (!ov || !video || !tilesRow) return
      const ovCtx = ov.getContext('2d')
      if (!ovCtx) return

      ovCtx.drawImage(video, 0, 0, W, H)
      const result = resultRef.current
      let count = 0

      if (result?.persons) {
        count = result.persons.length
        const tiles = tilesRef.current

        while (tiles.length < count) {
          const i = tiles.length
          const wrap = document.createElement('div')
          wrap.style.cssText = 'position:relative;flex:0 0 auto;border-radius:10px;overflow:hidden;background:#111'
          const num = document.createElement('div')
          num.style.cssText = 'position:absolute;top:6px;left:8px;background:rgba(0,0,0,0.6);font-size:12px;padding:2px 8px;border-radius:4px'
          num.textContent = `#${i + 1}`
          const gl = document.createElement('div')
          gl.style.cssText = 'position:absolute;bottom:6px;left:8px;right:8px;font-size:12px;font-weight:600;padding:3px 8px;border-radius:4px;text-align:center;background:rgba(0,0,0,0.7)'
          gl.textContent = ''
          gestureLabelsRef.current.set(i, gl)
          const c = document.createElement('canvas')
          wrap.append(c, num, gl)
          tilesRow.appendChild(wrap)
          tiles.push({ wrap, canvas: c, ctx: c.getContext('2d')! })
        }
        while (tiles.length > count) {
          tiles.pop()!.wrap.remove()
        }


        for (let i = 0; i < count; i++) {
          const p = result.persons[i]
          const [cx, cy, w, h] = p.bbox
          const color = COLORS[i % COLORS.length]
          const t = tiles[i]

          let sx = (cx - w / 2) * W
          let sy = (cy - h / 2) * H
          let sw = w * W
          let sh = h * H
          if (sx < 0) { sw += sx; sx = 0 }
          if (sy < 0) { sh += sy; sy = 0 }
          if (sx + sw > W) sw = W - sx
          if (sy + sh > H) sh = H - sy

          if (sw >= 4 && sh >= 4 && p.keypoints && p.keypoints.length >= 23) {
            const tileW = 240
            const tileH = Math.max(80, Math.round(tileW * (sh / sw)))
            t.canvas.width = tileW
            t.canvas.height = tileH
            t.wrap.style.height = tileH + 'px'
            t.ctx.drawImage(ov, sx, sy, sw, sh, 0, 0, tileW, tileH)
            const leftNorm = sx / W
            const topNorm = sy / H
            const visW_ = sw / W
            const visH_ = sh / H
            const tileKp = p.keypoints.map(kp => [
              ((kp[0] - leftNorm) / visW_),
              ((kp[1] - topNorm) / visH_),
              kp[2],
            ] as number[])
            drawSkeleton(t.ctx, tileKp, tileW, tileH, color)
          }

          // gesture detection
          const now = Date.now()
          const kp = p.keypoints
          let gesture: Gesture = 'none'
          console.log(`[gesture] person=${i} hasKp=${!!kp} kpLen=${kp?.length}`)
          if (kp && kp.length >= 23) {
            let rHist = rWristHistRef.current.get(i) || []
            rHist.push({ x: kp[16][0], y: kp[16][1] })
            if (rHist.length > 30) rHist = rHist.slice(-30)
            rWristHistRef.current.set(i, rHist)

            let lHist = lWristHistRef.current.get(i) || []
            lHist.push({ x: kp[15][0], y: kp[15][1] })
            if (lHist.length > 30) lHist = lHist.slice(-30)
            lWristHistRef.current.set(i, lHist)

            gesture = detectGesture(kp, rHist, lHist)
            console.log(`[gesture] person=${i} detected=${gesture} nose.y=${kp[0][1].toFixed(3)} rWrist.y=${kp[16][1].toFixed(3)} lWrist.y=${kp[15][1].toFixed(3)} nose.y-0.15=${(kp[0][1]-0.15).toFixed(3)}`)
            if (gesture !== 'none') {
              lastActiveRef.current.set(i, now)
            }
          }

          const waiting = shouldShowWaiting(gesture, lastActiveRef.current.get(i) ?? 0, now)
          const final = waiting ? 'waiting' : gesture
          console.log(`[gesture] person=${i} final=${final}`)
          const info = gestureInfo(final)
          const gl = gestureLabelsRef.current.get(i)
          if (gl) {
            gl.textContent = info.label
            gl.style.color = info.color
            gl.style.border = `1px solid ${info.color}`
            console.log(`[gesture] person=${i} labelEl updated to "${info.label}"`)
          } else {
            console.log(`[gesture] person=${i} NO LABEL EL FOUND`)
          }

          const bx = (cx - w / 2) * W
          const by = (cy - h / 2) * H
          const bw2 = w * W
          const bh2 = h * H
          ovCtx.strokeStyle = color
          ovCtx.lineWidth = 2
          ovCtx.strokeRect(bx, by, bw2, bh2)

          if (p.keypoints && p.keypoints.length >= 23) {
            drawSkeleton(ovCtx, p.keypoints, W, H, color)
          }
        }
      }
    }
    render()
    return () => cancelAnimationFrame(animRef.current)
  }, [])

  return (
    <div style={{ display:'flex', flexDirection:'column', alignItems:'center', gap:16, padding:20, height:'100vh' }}>
      <div style={{ position:'relative', width:'100%', maxWidth:960, borderRadius:12, overflow:'hidden', background:'#111', flexShrink:0 }}>
        <video ref={videoRef} autoPlay playsInline style={{ display:'block', width:'100%', borderRadius:12 }} />
        <canvas ref={overlayRef} style={{ position:'absolute', inset:0, width:'100%', height:'100%', pointerEvents:'none' }} />
        <div style={{ position:'absolute', top:12, left:12, background:'rgba(0,0,0,0.7)', backdropFilter:'blur(8px)', padding:'6px 16px', borderRadius:8, fontSize:15, fontWeight:600, pointerEvents:'none' }}>
          Persons: <span style={{ color:'#4ade80' }}>{personCount}</span>
        </div>
        <div style={{ position:'absolute', top:12, right:12, padding:'6px 12px', borderRadius:8, fontSize:13, fontWeight:500, background:'rgba(0,0,0,0.7)', backdropFilter:'blur(8px)', color: connected ? '#4ade80' : '#f87171' }}>
          {connected ? '● online' : '● offline'}
        </div>
      </div>
      <div ref={tilesRowRef} style={{ display:'flex', gap:12, flexWrap:'wrap', width:'100%', maxWidth:960, minHeight:80 }} />
    </div>
  )
}
