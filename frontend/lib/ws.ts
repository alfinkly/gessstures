'use client'

import { useEffect, useRef, useCallback } from 'react'

function wsUrl(): string {
  if (typeof window === 'undefined') return 'ws://localhost:3030'
  const host = window.location.hostname
  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  // local dev → connect directly to ws-bridge on :3030
  if (host === 'localhost' || host === '127.0.0.1' || host.match(/^192\.168\./)) {
    return `${proto}//${host}:3030`
  }
  // remote / cloudflare tunnel → go through same port via /ws path
  const port = window.location.port || (window.location.protocol === 'https:' ? '443' : '80')
  return `${proto}//${host}:${port}/ws`
}

export type GraphSnapshot = {
  nodes: { index: number; id: string; label: string; x: number; y: number; z: number; degree: number }[]
  edges: { from: number; to: number; weight: number }[]
}

export type BodyResult = {
  person_count: number
  persons: { bbox: [number, number, number, number]; keypoints: [number, number, number][] }[]
}

export type WsConnection = {
  send: (msg: unknown) => void
  sendBinary: (data: ArrayBuffer) => void
}

export function useWs(
  onSnapshot: (s: GraphSnapshot) => void,
  onBody: (r: BodyResult) => void,
) {
  const wsRef = useRef<WebSocket | null>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval>>(undefined)

  useEffect(() => {
    let reconnectTimer: ReturnType<typeof setTimeout>

    function connect() {
      const ws = new WebSocket(wsUrl())
      ws.binaryType = 'arraybuffer'
      wsRef.current = ws

      ws.onmessage = (e) => {
        if (typeof e.data !== 'string') return
        try {
          const data = JSON.parse(e.data)
          if (data.person_count !== undefined) {
            console.log(`[ws] body result: ${data.person_count} persons, keypoints=${data.persons?.[0]?.keypoints?.length ?? 'none'}`)
            onBody(data as BodyResult)
          }
          else if (data.nodes) onSnapshot(data as GraphSnapshot)
        } catch {}
      }

      ws.onclose = () => {
        reconnectTimer = setTimeout(connect, 2000)
      }
    }

    connect()
    return () => {
      clearTimeout(reconnectTimer)
      wsRef.current?.close()
      clearInterval(intervalRef.current)
    }
  }, [onSnapshot, onBody])

  const send = useCallback((msg: unknown) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg))
    }
  }, [])

  const sendBinary = useCallback((data: ArrayBuffer) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(data)
    }
  }, [])

  return { send, sendBinary }
}
