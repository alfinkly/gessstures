'use client'

import { useEffect, useRef } from 'react'
import * as THREE from 'three'
import { OrbitControls } from 'three/addons/controls/OrbitControls.js'
import type { GraphSnapshot } from '@/lib/ws'

const NODE_COLOR = 0x6366f1
const EDGE_COLOR = 0x334155

export default function GraphCanvas({ onReady }: { onReady?: () => void }) {
  const containerRef = useRef<HTMLDivElement>(null)
  const sceneRef = useRef<{
    scene: THREE.Scene; camera: THREE.PerspectiveCamera; renderer: THREE.WebGLRenderer
    controls: OrbitControls; nodes: Map<number, THREE.Mesh>; edges: Map<string, any>
    labelSprites: Map<number, THREE.Sprite>; nodeGroup: THREE.Group; edgeGroup: THREE.Group
    nodeGeom: THREE.SphereGeometry; nodeMat: THREE.MeshStandardMaterial
  }>(null)

  useEffect(() => {
    if (!containerRef.current) return
    const el = containerRef.current

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false })
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    renderer.setClearColor(0x0a0a0f)
    el.appendChild(renderer.domElement)

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(55, el.clientWidth / el.clientHeight, 0.5, 500)
    camera.position.set(15, 12, 20)

    const controls = new OrbitControls(camera, renderer.domElement)
    controls.enableDamping = true
    controls.dampingFactor = 0.08
    controls.target.set(0, 0, 0)
    controls.update()

    scene.add(new THREE.AmbientLight(0x404060, 1.2))
    const dir = new THREE.DirectionalLight(0xffffff, 0.8)
    dir.position.set(10, 20, 15)
    scene.add(dir)

    const nodeGroup = new THREE.Group()
    const edgeGroup = new THREE.Group()
    scene.add(nodeGroup)
    scene.add(edgeGroup)

    const nodeGeom = new THREE.SphereGeometry(0.35, 24, 16)
    const nodeMat = new THREE.MeshStandardMaterial({ color: NODE_COLOR, roughness: 0.4, metalness: 0.2 })

    const state = {
      scene, camera, renderer, controls,
      nodes: new Map<number, THREE.Mesh>(),
      edges: new Map<string, THREE.Line>(),
      labelSprites: new Map<number, THREE.Sprite>(),
      nodeGroup, edgeGroup, nodeGeom, nodeMat,
    }
    sceneRef.current = state as any

    function resize() {
      const w = el.clientWidth; const h = el.clientHeight
      renderer.setSize(w, h)
      camera.aspect = w / h
      camera.updateProjectionMatrix()
    }
    window.addEventListener('resize', resize)
    resize()
    onReady?.()

    function animate() {
      requestAnimationFrame(animate)
      controls.update()
      renderer.render(scene, camera)
    }
    animate()

    return () => {
      window.removeEventListener('resize', resize)
      renderer.dispose()
      el.removeChild(renderer.domElement)
    }
  }, [onReady])

  // Expose updateGraph on window so we can call it from the page
  useEffect(() => {
    const w = window as any
    w.__updateGraph = (snapshot: GraphSnapshot) => {
      const s = sceneRef.current
      if (!s) return
      const { nodes, edges, labelSprites, nodeGroup, edgeGroup, nodeGeom, nodeMat } = s as any

      const existingIds = new Set<number>()
      for (const n of snapshot.nodes) {
        existingIds.add(n.index)
        let mesh = nodes.get(n.index) as THREE.Mesh | undefined
        if (!mesh) {
          mesh = new THREE.Mesh(nodeGeom, nodeMat.clone())
          nodeGroup.add(mesh)
          nodes.set(n.index, mesh)
          const label = createLabel(n.label)
          mesh.add(label)
          labelSprites.set(n.index, label)
        }
        const scale = 0.5 + Math.min(n.degree, 10) * 0.12
        mesh.scale.setScalar(scale)
        mesh.position.set(n.x, n.y, n.z)
      }


      for (const [idx, mesh] of nodes) {
        if (!existingIds.has(idx)) {
          nodeGroup.remove(mesh)
          mesh.geometry && mesh.geometry.dispose()
          mesh.material.dispose()
          nodes.delete(idx)
          const sp = labelSprites.get(idx)
          if (sp) { sp.material.map?.dispose(); sp.material.dispose(); labelSprites.delete(idx) }
        }
      }

      const edgeKey = (a: number, b: number) => `${Math.min(a, b)}-${Math.max(a, b)}`
      const existingEdges = new Set<string>()

      for (const e of snapshot.edges) {
        const key = edgeKey(e.from, e.to)
        existingEdges.add(key)
        if (edges.has(key)) continue
        const fromNode = nodes.get(e.from)
        const toNode = nodes.get(e.to)
        if (!fromNode || !toNode) continue
        const pts = [fromNode.position.clone(), toNode.position.clone()]
        const geom = new THREE.BufferGeometry().setFromPoints(pts)
        const mat = new THREE.LineBasicMaterial({ color: EDGE_COLOR, transparent: true, opacity: 0.35 })
        const line = new THREE.Line(geom, mat)
        edgeGroup.add(line)
        edges.set(key, { line, from: e.from, to: e.to })
      }

      for (const [key, edge] of edges) {
        if (!existingEdges.has(key)) {
          edgeGroup.remove(edge.line)
          edge.line.geometry.dispose()
          edge.line.material.dispose()
          edges.delete(key)
        } else {
          const fromNode = nodes.get(edge.from)
          const toNode = nodes.get(edge.to)
          if (fromNode && toNode) {
            const pos = edge.line.geometry.attributes.position
            pos.setXYZ(0, fromNode.position.x, fromNode.position.y, fromNode.position.z)
            pos.setXYZ(1, toNode.position.x, toNode.position.y, toNode.position.z)
            pos.needsUpdate = true
          }
        }
      }
    }

    return () => { delete (window as any).__updateGraph }
  }, [])

  return <div ref={containerRef} style={{ width: '100%', height: '100%' }} />
}

function createLabel(text: string) {
  const canvas = document.createElement('canvas')
  const size = 256
  canvas.width = size
  canvas.height = size / 2
  const ctx = canvas.getContext('2d')!
  ctx.fillStyle = '#e2e8f0'
  ctx.font = 'bold 32px -apple-system, BlinkMacSystemFont, sans-serif'
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  let display = text
  if (ctx.measureText(display).width > size - 20) {
    while (display.length > 1 && ctx.measureText(display + '…').width > size - 20) display = display.slice(0, -1)
    display += '…'
  }
  ctx.fillText(display, size / 2, size / 4)
  const texture = new THREE.CanvasTexture(canvas)
  texture.minFilter = THREE.LinearFilter
  const spriteMat = new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false })
  const sprite = new THREE.Sprite(spriteMat)
  sprite.scale.set(3, 1.5, 1)
  sprite.position.set(0, 1.0, 0)
  return sprite
}
