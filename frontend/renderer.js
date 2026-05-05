import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const NODE_COLOR = 0x6366f1;
const NODE_HIGHLIGHT = 0xa78bfa;
const EDGE_COLOR = 0x334155;
const BG_COLOR = 0x0a0a0f;
const LABEL_COLOR = '#e2e8f0';

export class GraphRenderer {
  constructor(canvas) {
    this.canvas = canvas;

    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: false });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setClearColor(BG_COLOR);

    this.scene = new THREE.Scene();

    this.camera = new THREE.PerspectiveCamera(55, 2, 0.5, 500);
    this.camera.position.set(15, 12, 20);
    this.camera.lookAt(0, 0, 0);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.target.set(0, 0, 0);
    this.controls.update();

    const ambient = new THREE.AmbientLight(0x404060, 1.2);
    this.scene.add(ambient);

    const dir = new THREE.DirectionalLight(0xffffff, 0.8);
    dir.position.set(10, 20, 15);
    this.scene.add(dir);

    this.nodeGroup = new THREE.Group();
    this.edgeGroup = new THREE.Group();
    this.scene.add(this.nodeGroup);
    this.scene.add(this.edgeGroup);

    this.nodes = new Map();
    this.edges = new Map();
    this.labelSprites = new Map();

    this.nodeGeom = new THREE.SphereGeometry(0.35, 24, 16);
    this.nodeMaterial = new THREE.MeshStandardMaterial({
      color: NODE_COLOR,
      roughness: 0.4,
      metalness: 0.2,
    });
    this.nodeMaterialHighlight = new THREE.MeshStandardMaterial({
      color: NODE_HIGHLIGHT,
      roughness: 0.3,
      metalness: 0.3,
    });

    this.onResize = this.onResize.bind(this);
    window.addEventListener('resize', this.onResize);
    this.onResize();
  }

  onResize() {
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.renderer.setSize(w, h, false);
      this.camera.aspect = w / Math.max(h, 1);
      this.camera.updateProjectionMatrix();
    }
  }

  updateGraph(snapshot) {
    const existingIds = new Set();

    for (const node of snapshot.nodes) {
      existingIds.add(node.index);

      let mesh = this.nodes.get(node.index);
      if (!mesh) {
        const mat = node.degree > 0
          ? this.nodeMaterial.clone()
          : new THREE.MeshStandardMaterial({
              color: 0x475569,
              roughness: 0.6,
              metalness: 0.1,
            });
        mesh = new THREE.Mesh(this.nodeGeom, mat);
        mesh.castShadow = false;
        mesh.receiveShadow = false;
        this.nodeGroup.add(mesh);
        this.nodes.set(node.index, mesh);

        const label = this.createLabel(node.label);
        mesh.add(label);
        this.labelSprites.set(node.index, label);
      }

      const scale = 0.5 + Math.min(node.degree, 10) * 0.12;
      mesh.scale.setScalar(scale);
      mesh.position.set(node.x, node.y, node.z);
    }

    for (const [idx, mesh] of this.nodes) {
      if (!existingIds.has(idx)) {
        this.nodeGroup.remove(mesh);
        mesh.geometry && mesh.geometry.dispose();
        mesh.material.dispose();
        this.nodes.delete(idx);
        const sprite = this.labelSprites.get(idx);
        if (sprite) {
          sprite.material.map.dispose();
          sprite.material.dispose();
          this.labelSprites.delete(idx);
        }
      }
    }

    const edgeKey = (a, b) => `${Math.min(a, b)}-${Math.max(a, b)}`;
    const existingEdges = new Set();

    for (const edge of snapshot.edges) {
      const key = edgeKey(edge.from, edge.to);
      existingEdges.add(key);

      if (this.edges.has(key)) continue;

      const fromNode = this.nodes.get(edge.from);
      const toNode = this.nodes.get(edge.to);
      if (!fromNode || !toNode) continue;

      const points = [fromNode.position.clone(), toNode.position.clone()];
      const geom = new THREE.BufferGeometry().setFromPoints(points);
      const mat = new THREE.LineBasicMaterial({
        color: EDGE_COLOR,
        transparent: true,
        opacity: 0.35,
        linewidth: 1,
      });
      const line = new THREE.Line(geom, mat);
      this.edgeGroup.add(line);
      this.edges.set(key, { line, from: edge.from, to: edge.to });
    }

    for (const [key, edge] of this.edges) {
      if (!existingEdges.has(key)) {
        this.edgeGroup.remove(edge.line);
        edge.line.geometry.dispose();
        edge.line.material.dispose();
        this.edges.delete(key);
      } else {
        const fromNode = this.nodes.get(edge.from);
        const toNode = this.nodes.get(edge.to);
        if (fromNode && toNode) {
          const positions = edge.line.geometry.attributes.position;
          positions.setXYZ(0, fromNode.position.x, fromNode.position.y, fromNode.position.z);
          positions.setXYZ(1, toNode.position.x, toNode.position.y, toNode.position.z);
          positions.needsUpdate = true;
        }
      }
    }
  }

  createLabel(text) {
    const canvas = document.createElement('canvas');
    const size = 256;
    canvas.width = size;
    canvas.height = size / 2;
    const ctx = canvas.getContext('2d');

    ctx.fillStyle = LABEL_COLOR;
    ctx.font = 'bold 32px -apple-system, BlinkMacSystemFont, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    const maxWidth = size - 20;
    let display = text;
    if (ctx.measureText(display).width > maxWidth) {
      while (display.length > 1 && ctx.measureText(display + '…').width > maxWidth) {
        display = display.slice(0, -1);
      }
      display += '…';
    }
    ctx.fillText(display, size / 2, size / 4);

    const texture = new THREE.CanvasTexture(canvas);
    texture.minFilter = THREE.LinearFilter;

    const spriteMat = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
    });
    const sprite = new THREE.Sprite(spriteMat);
    sprite.scale.set(3, 1.5, 1);
    sprite.position.set(0, 1.0, 0);
    return sprite;
  }

  render() {
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
  }
}
