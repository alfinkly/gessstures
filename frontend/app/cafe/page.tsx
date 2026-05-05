'use client'

import { useEffect, useState, useRef, useMemo } from 'react'

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

// ── Body Detect Component ────────────────────────────────────
function CameraBodyDetect() {
  const videoRef = useRef<HTMLVideoElement>(null)
  const overlayRef = useRef<HTMLCanvasElement>(null)
  const tilesRowRef = useRef<HTMLDivElement>(null)
  const bodyResultRef = useRef<any>(null)
  const tilesRef = useRef<{ wrap: HTMLDivElement; canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D }[]>([])
  const [personCount, setPersonCount] = useState(0)
  const animRef = useRef(0)
  const tilesAnimRef = useRef<number>(0)

  useEffect(() => {
    if (overlayRef.current) { overlayRef.current.width = W; overlayRef.current.height = H }
  }, [])

  useEffect(() => {
    let stream: MediaStream | null = null
    navigator.mediaDevices.getUserMedia({ video: { width: { exact: W }, height: { exact: H }, facingMode: 'user' }, audio: false })
      .then(s => { stream = s; if (videoRef.current) videoRef.current.srcObject = s })
      .catch(() => {})
    return () => { stream?.getTracks().forEach(t => t.stop()) }
  }, [])

  const capRef = useRef<HTMLCanvasElement | null>(null)
  useEffect(() => {
    const c = document.createElement('canvas'); c.width = W; c.height = H; capRef.current = c
  }, [])

  // own WebSocket for sending frames + receiving body results
  useEffect(() => {
    const wsUrl = (() => {
      const host = window.location.hostname
      const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
      if (host === 'localhost' || host === '127.0.0.1' || host.match(/^192\.168\./)) return `${proto}//${host}:3030`
      return `${proto}//${host}:${window.location.port || '443'}/ws`
    })()
    let ws: WebSocket | null = null
    let reconnectTimer: ReturnType<typeof setTimeout>
    const cap = document.createElement('canvas'); cap.width = W; cap.height = H

    let frameCount = 0
    function connect() {
      ws = new WebSocket(wsUrl)
      ws.onopen = () => console.log('[bodycam] WS connected')
      ws.onmessage = (e) => {
        if (typeof e.data !== 'string') return
        try {
          const d = JSON.parse(e.data)
          console.log('[bodycam] msg keys:', Object.keys(d), 'person_count:', d.person_count)
          if (d.person_count !== undefined) {
            bodyResultRef.current = d
            setPersonCount(d.person_count ?? 0)
            console.log('[bodycam] calling __bodyDetect(', d.person_count, ')')
            ;(window as any).__bodyDetect?.(d.person_count)
          }
        } catch {}
      }
      ws.onclose = () => {
        console.log('[bodycam] WS closed, reconnecting in 2s')
        reconnectTimer = setTimeout(connect, 2000)
      }
    }

    connect()

    const int = setInterval(() => {
      const v = videoRef.current
      if (!v || !ws || ws.readyState !== WebSocket.OPEN) return
      const ctx = cap.getContext('2d'); if (!ctx) return
      ctx.drawImage(v, 0, 0, W, H)
      frameCount++
      cap.toBlob(blob => {
        if (blob) {
          console.log('[bodycam] sending frame', frameCount)
          blob.arrayBuffer().then(buf => ws?.send(buf))
        }
      }, 'image/jpeg', 0.8)
    }, 200)
    return () => { clearTimeout(reconnectTimer); clearInterval(int); ws?.close() }
  }, [])

  // render loop for overlay + tiles
  useEffect(() => {
    let renderCount = 0
    function render() {
      animRef.current = requestAnimationFrame(render)
      renderCount++
      const ov = overlayRef.current; const v = videoRef.current; const tr = tilesRowRef.current
      if (!ov || !v || !tr) return
      const ctx = ov.getContext('2d'); if (!ctx) return

      try {
        ctx.drawImage(v, 0, 0, W, H)
      } catch (e) {
        console.log('[render] drawImage error:', e)
        return
      }

      const result = bodyResultRef.current

      if (renderCount % 30 === 0) {
        console.log('[render] frame', renderCount, 'result:', result ? 'persons=' + result.persons?.length + ' count=' + result.person_count : 'null')
      }

      let count = 0

      if (result?.persons) {
        count = result.persons.length
        const tiles = tilesRef.current

        while (tiles.length < count) {
          const i = tiles.length
          const wrap = document.createElement('div')
          wrap.style.cssText = 'position:relative;flex:0 0 auto;border-radius:10px;overflow:hidden;background:#111'
          const num = document.createElement('div')
          num.style.cssText = 'position:absolute;top:4px;left:6px;background:rgba(0,0,0,0.6);font-size:11px;padding:2px 6px;border-radius:4px'
          num.textContent = `#${i + 1}`
          const c = document.createElement('canvas')
          wrap.append(c, num)
          tr.appendChild(wrap)
          tiles.push({ wrap, canvas: c, ctx: c.getContext('2d')! })
        }
        while (tiles.length > count) { tiles.pop()!.wrap.remove() }

        for (let i = 0; i < count; i++) {
          const p = result.persons[i]
          const [cx, cy, w, h] = p.bbox
          const color = COLORS[i % COLORS.length]

          let sx = (cx - w / 2) * W; let sy = (cy - h / 2) * H
          let sw = w * W; let sh = h * H
          if (sx < 0) { sw += sx; sx = 0 }
          if (sy < 0) { sh += sy; sy = 0 }
          if (sx + sw > W) sw = W - sx
          if (sy + sh > H) sh = H - sy

          if (sw >= 4 && sh >= 4 && p.keypoints && p.keypoints.length >= 23) {
            const tileW = 180; const tileH = Math.max(60, Math.round(tileW * (sh / sw)))
            const t = tiles[i]
            t.canvas.width = tileW; t.canvas.height = tileH
            t.wrap.style.height = tileH + 'px'
            t.ctx.drawImage(ov, sx, sy, sw, sh, 0, 0, tileW, tileH)
            const ln = sx / W; const tn = sy / H
            const vw = sw / W; const vh = sh / H
            const tkp = (p.keypoints as number[][]).map(pt => ([
              ((pt[0] - ln) / vw),
              ((pt[1] - tn) / vh),
              pt[2],
            ]))
            drawSkeleton(t.ctx, tkp, tileW, tileH, color)
          }

          ctx.strokeStyle = color; ctx.lineWidth = 2
          ctx.strokeRect((cx - w/2)*W, (cy - h/2)*H, w*W, h*H)
          if (p.keypoints && p.keypoints.length >= 23) {
            drawSkeleton(ctx, p.keypoints as number[][], W, H, color)
          }
        }
      }
    }
    render()
    return () => cancelAnimationFrame(animRef.current)
  }, [])

  return (
    <div>
      <div style={{ position: 'relative', width: '100%', maxWidth: 720, borderRadius: 12, overflow: 'hidden', background: '#111' }}>
        <video ref={videoRef} autoPlay playsInline muted style={{ display: 'block', width: '100%' }} />
        <canvas ref={overlayRef} style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', pointerEvents: 'none' }} />
      </div>
      <div ref={tilesRowRef} style={{ display: 'flex', gap: 8, flexWrap: 'wrap', width: '100%', maxWidth: 720, minHeight: 50, marginTop: 8 }} />
    </div>
  )
}

// ── Format helpers ────────────────────────────────────────────────
function fmtDuration(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${Math.round(secs % 60)}s`
  const h = Math.floor(secs / 3600); const m = Math.floor((secs % 3600) / 60)
  return `${h}h ${m}m`
}

function fmtTime(ts: number): string {
  const d = new Date(ts * 1000)
  return d.toLocaleTimeString('ru-RU', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

interface PersonData {
  id: number; first_seen: number; last_seen: number; total_seen_secs: number
  visit_count: number; face_jpeg_b64: string
  visits: { start: number; end: number; camera_id: string }[]
  current: boolean
}

// ── Person Card ────────────────────────────────────────────────────
function PersonCard({ p, selected, onClick }: { p: PersonData; selected: boolean; onClick: () => void }) {
  const faceUrl = p.face_jpeg_b64
    ? (p.face_jpeg_b64.startsWith('data:') ? p.face_jpeg_b64 : `data:image/jpeg;base64,${p.face_jpeg_b64}`)
    : null
  return (
    <div
      onClick={onClick}
      style={{
        display: 'flex', gap: 12, alignItems: 'center', padding: '10px 14px',
        borderRadius: 10, cursor: 'pointer', transition: 'all 0.15s',
        background: selected ? 'rgba(99,102,241,0.15)' : 'rgba(255,255,255,0.04)',
        border: selected ? '1px solid #6366f1' : '1px solid transparent',
      }}
    >
      <div style={{
        width: 52, height: 52, borderRadius: 8, overflow: 'hidden', background: '#1e1e2a', flexShrink: 0,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: 20, color: '#6366f1', fontWeight: 700,
      }}>
        {faceUrl ? <img src={faceUrl} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : '👤'}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontWeight: 600, fontSize: 14 }}>Person #{p.id}</div>
        <div style={{ fontSize: 12, color: '#94a3b8', marginTop: 2 }}>
          {fmtDuration(p.total_seen_secs)} total &middot; {p.visit_count} visits
        </div>
        <div style={{ fontSize: 11, color: '#64748b', marginTop: 1 }}>
          last seen {fmtTime(p.last_seen)}
        </div>
      </div>
      {p.current ? <span style={{ width: 8, height: 8, borderRadius: '50%', background: '#4ade80', flexShrink: 0 }} /> : null}
    </div>
  )
}

// ── Timeline for one person ────────────────────────────────────────
function PersonTimeline({ p }: { p: PersonData }) {
  return (
    <div style={{ marginTop: 8 }}>
      <div style={{ fontSize: 13, fontWeight: 600, color: '#94a3b8', marginBottom: 8 }}>
        Timeline — Person #{p.id}
      </div>
      {p.visits.length === 0 ? (
        <div style={{ fontSize: 12, color: '#64748b', fontStyle: 'italic' }}>Still in view…</div>
      ) : (
        p.visits.map((v, i) => (
          <div key={i} style={{
            display: 'flex', gap: 12, alignItems: 'center', padding: '6px 0',
            borderBottom: '1px solid rgba(255,255,255,0.05)', fontSize: 12,
          }}>
            <span style={{ color: '#6366f1', fontWeight: 600, minWidth: 60 }}>
              Visit #{i + 1}
            </span>
            <span style={{ color: '#e2e8f0' }}>{fmtTime(v.start)}</span>
            <span style={{ color: '#64748b' }}>→</span>
            <span style={{ color: '#e2e8f0' }}>{fmtTime(v.end)}</span>
            <span style={{ color: '#94a3b8', marginLeft: 'auto' }}>
              {fmtDuration(v.end - v.start)} at {v.camera_id}
            </span>
          </div>
        ))
      )}
      <div style={{ marginTop: 10, fontSize: 13, color: '#64748b' }}>
        Total: <span style={{ color: '#e2e8f0' }}>{fmtDuration(p.total_seen_secs)}</span>
      </div>
    </div>
  )
}

// ── Video Player for a person ──────────────────────────────────────
function PersonVideo({ p }: { p: PersonData }) {
  // For MVP, show face crop sequence as a "video" using canvas animation
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [playing, setPlaying] = useState(false)
  const frameIdx = useRef(0)
  const animRef = useRef(0)

  useEffect(() => {
    if (!playing || !p.visits.length) return
    // collect all face snapshots (from the latest visit)
    const allSnaps = p.visits.flatMap(v => v as any)
    if (!allSnaps.length) return

    let i = 0
    function play() {
      if (!canvasRef.current) return
      const ctx = canvasRef.current.getContext('2d')
      if (!ctx) return
      // draw placeholder — in real impl, fetch face JPEGs from server
      ctx.fillStyle = '#1e1e2a'
      ctx.fillRect(0, 0, 320, 240)
      ctx.fillStyle = '#6366f1'
      ctx.font = '14px sans-serif'
      ctx.fillText(`Person #${p.id} — frame ${i}`, 16, 120)
      i++
      if (i > 60) i = 0
      animRef.current = requestAnimationFrame(play)
    }
    play()
    return () => cancelAnimationFrame(animRef.current)
  }, [playing, p])

  return (
    <div style={{ marginTop: 12 }}>
      <div style={{ fontSize: 13, fontWeight: 600, color: '#94a3b8', marginBottom: 6 }}>
        Compilation — Person #{p.id}
      </div>
      <canvas ref={canvasRef} width={320} height={240}
        style={{ width: '100%', maxWidth: 400, borderRadius: 8, background: '#1e1e2a', cursor: 'pointer' }}
        onClick={() => setPlaying(!playing)}
      />
      <div style={{ fontSize: 11, color: '#64748b', marginTop: 4 }}>
        Click to {playing ? 'pause' : 'play'} compilation
      </div>
    </div>
  )
}

// ── Main Page ──────────────────────────────────────────────────────
export const dynamic = 'force-dynamic'

export default function CafePage() {
  const [people, setPeople] = useState<PersonData[]>([])
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const wsConnectedRef = useRef(false)

  const [localStats, setLocalStats] = useState({ total: 0, inView: 0, visits: 0 })
  const localPeopleRef = useRef<Map<number, {
    first: number; last: number; totalSecs: number; current: boolean; faceJpeg: string
    visitHistory: { start: number; end: number }[]
  }>>(new Map())
  const bodyCountRef = useRef(0)
  const bodyTimerRef = useRef<ReturnType<typeof setInterval>>(undefined)

  useEffect(() => {
    (window as any).__bodyDetect = (person_count: number) => {
      console.log('[local] __bodyDetect called with', person_count, 'people')
      const now = Date.now() / 1000
      bodyCountRef.current = person_count
      // capture who was current before reset
      const wasCurrent = new Map<number, boolean>()
      for (const [id, p] of localPeopleRef.current) wasCurrent.set(id, p.current)
      for (const p of localPeopleRef.current.values()) p.current = false
      for (let i = 0; i < person_count; i++) {
        const existing = localPeopleRef.current.get(i)
        if (existing) {
          const delta = now - existing.last
          existing.totalSecs += delta
          existing.last = now
          existing.current = true
          console.log('[local] update #' + i + ': +' + delta.toFixed(2) + 's = ' + existing.totalSecs.toFixed(2) + 's')
          if (wasCurrent.get(i)) {
            // still in view — extend current visit
            const v = existing.visitHistory[existing.visitHistory.length - 1]
            if (v) v.end = now
          } else {
            // came back — start new visit
            existing.visitHistory.push({ start: now, end: now })
          }
        } else {
          console.log('[local] create #' + i)
          localPeopleRef.current.set(i, {
            first: now, last: now, totalSecs: 0, current: true,
            faceJpeg: '',
            visitHistory: [{ start: now, end: now }],
          })
        }
      }
    }
    // compute stats every second
    bodyTimerRef.current = setInterval(() => {
      const now = Date.now() / 1000
      let total = 0, inView = 0
      for (const p of localPeopleRef.current.values()) {
        total++
        if (p.current) inView++
      }
      setLocalStats({ total: Math.max(total, localPeopleRef.current.size), inView, visits: total })
    }, 1000)
    return () => { clearInterval(bodyTimerRef.current); delete (window as any).__bodyDetect }
  }, [])

  useEffect(() => {
    const w = window as any
    w.__camConnected = (v: boolean) => { wsConnectedRef.current = v }
    w.__peopleData = (list: PersonData[]) => {
      setPeople(list)
    }
    return () => { delete (window as any).__camConnected; delete (window as any).__peopleData }
  }, [])

  useEffect(() => {
    const host = typeof window !== 'undefined' ? window.location.hostname : 'localhost'
    const proto = typeof window !== 'undefined' && window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    let wsAddr: string
    if (host === 'localhost' || host === '127.0.0.1' || host.match(/^192\.168\./)) {
      wsAddr = `${proto}//${host}:3030`
    } else {
      const port = window.location.port || (window.location.protocol === 'https:' ? '443' : '80')
      wsAddr = `${proto}//${host}:${port}/ws`
    }
    let ws: WebSocket | null = null
    let reconnectTimer: ReturnType<typeof setTimeout>

    function connect() {
      ws = new WebSocket(wsAddr)
      ws.onopen = () => {
        console.log('[cafe-ws] connected')
        ;(window as any).__camConnected?.(true)
      }
      ws.onmessage = (e) => {
        if (typeof e.data !== 'string') return
        try {
          const d = JSON.parse(e.data)
          console.log('[cafe-ws] msg keys:', Object.keys(d), 't:', d.t, 'person_count:', d.person_count)
          if (d.t === 'people' && d.d?.people) {
            console.log('[cafe-ws] people update:', d.d.people.length, 'people')
            setPeople(d.d.people)
          } else if (d.person_count !== undefined) {
            console.log('[cafe-ws] body result, calling __bodyDetect')
            ;(window as any).__bodyResult?.(d)
            ;(window as any).__bodyDetect?.(d.person_count)
          }
        } catch {}
      }
      ws.onclose = () => {
        console.log('[cafe-ws] closed, reconnecting in 2s')
        ;(window as any).__camConnected?.(false)
        reconnectTimer = setTimeout(connect, 2000)
      }
    }

    connect()
    return () => { clearTimeout(reconnectTimer); ws?.close() }
  }, [])

  const selected = people.find(p => p.id === selectedId) ?? null

  // merge face tracker people + local body fallback
  const mergedPeople = useMemo(() => {
    if (people.length > 0) return people
    const result: PersonData[] = []
    for (const [id, p] of localPeopleRef.current) {
      result.push({
        id: 1000 + id,
        first_seen: p.first,
        last_seen: p.last,
        total_seen_secs: p.totalSecs,
        visit_count: p.visitHistory.length,
        face_jpeg_b64: p.faceJpeg,
        visits: p.visitHistory.map(v => ({ start: v.start, end: v.end, camera_id: 'cam_0' })),
        current: p.current,
      })
    }
    // sort: current first, then by last_seen desc
    result.sort((a, b) => {
      if (a.current !== b.current) return a.current ? -1 : 1
      return b.last_seen - a.last_seen
    })
    return result
  }, [people, localStats])

  const selectedMerged = mergedPeople.find(p => p.id === selectedId) ?? null

  return (
    <div style={{ padding: 16, background: '#0a0a0f', minHeight: '100vh', color: '#e2e8f0' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700 }}>☕ Gessstures Cafe</h1>
        <a href="/cam" style={{ fontSize: 13, color: '#6366f1' }}>📷 Phone cam →</a>
      </div>

      {/* Body Detection */}
      <div style={{ marginBottom: 20 }}>
        <h2 style={{ fontSize: 14, fontWeight: 600, color: '#94a3b8', marginBottom: 8 }}>LIVE CAMERA</h2>
        <CameraBodyDetect />
      </div>

      {/* Stats */}
      <div style={{ display: 'flex', gap: 12, marginBottom: 20, flexWrap: 'wrap' }}>
        <StatBox label="Total people" value={localStats.total.toString()} />
        <StatBox label="Currently in view" value={localStats.inView.toString()} />
        <StatBox label="Total visits" value={localStats.visits.toString()} />
        <StatBox label="Avg duration" value={mergedPeople.length ? fmtDuration(mergedPeople.reduce((s, p) => s + p.total_seen_secs, 0) / mergedPeople.length) : localStats.total > 0 ? 'active' : '—'} />
      </div>

      <style>{`
        @media (min-width: 768px) { .cafe-main-row { flex-direction: row !important; } }
      `}</style>
      {/* Main content: people list + detail */}
      <div style={{ display: 'flex', gap: 16, flexDirection: 'column' }}
        className="cafe-main-row">
        {/* People list */}
        <div style={{ flex: 1, minWidth: 0, maxWidth: 380 }}>
          <h2 style={{ fontSize: 14, fontWeight: 600, color: '#94a3b8', marginBottom: 8 }}>PEOPLE</h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 400, overflowY: 'auto' }}>
            {people.length === 0 && localStats.total === 0 && (
              <div style={{ fontSize: 13, color: '#64748b', fontStyle: 'italic', padding: 12 }}>
                No people detected yet. Open camera and show faces.
              </div>
            )}
            {people.length === 0 && localStats.total > 0 && (
              <div style={{ fontSize: 13, color: '#64748b', fontStyle: 'italic', padding: 12 }}>
                Body detection active ({localStats.inView} in view) — face recognition loading…
              </div>
            )}
            {mergedPeople.map(p => (
              <PersonCard key={p.id} p={p} selected={selectedId === p.id} onClick={() => setSelectedId(p.id)} />
            ))}
          </div>
        </div>

        {/* Detail panel */}
        {selectedMerged && (
          <div style={{ flex: 2, minWidth: 0, padding: '0 4px', maxHeight: 'calc(100vh - 350px)', overflowY: 'auto' }}>
            <PersonTimeline p={selectedMerged} />
            <PersonVideo p={selectedMerged} />
          </div>
        )}
      </div>

      {/* Footer */}
      <div style={{ marginTop: 30, fontSize: 11, color: '#475569', textAlign: 'center' }}>
        Gessstures Cafe MVP &mdash; face recognition + people tracking
      </div>
    </div>
  )
}

function StatBox({ label, value }: { label: string; value: string }) {
  return (
    <div style={{
      background: 'rgba(255,255,255,0.04)', borderRadius: 10, padding: '10px 16px', minWidth: 100, flex: 1,
    }}>
      <div style={{ fontSize: 11, color: '#64748b', textTransform: 'uppercase', letterSpacing: 0.5 }}>{label}</div>
      <div style={{ fontSize: 22, fontWeight: 700, color: '#e2e8f0', marginTop: 2 }}>{value}</div>
    </div>
  )
}
