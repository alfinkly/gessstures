import { GraphRenderer } from './renderer.js';
import { connectWs } from './ws-client.js';

const canvas = document.getElementById('canvas');
const statusEl = document.getElementById('ws-status');
const nodeCountEl = document.getElementById('node-count');
const fpsEl = document.getElementById('fps');

const renderer = new GraphRenderer(canvas);

let frameCount = 0;
let lastFpsTime = performance.now();

function updateFps() {
  frameCount++;
  const now = performance.now();
  if (now - lastFpsTime >= 1000) {
    fpsEl.textContent = `${frameCount} fps`;
    frameCount = 0;
    lastFpsTime = now;
  }
}

function onSnapshot(snapshot) {
  renderer.updateGraph(snapshot);
  nodeCountEl.textContent = `${snapshot.nodes.length} nodes`;
  updateFps();
}

function onConnected() {
  statusEl.textContent = '● connected';
  statusEl.className = 'connected';
}

function onDisconnected() {
  statusEl.textContent = '● disconnected';
  statusEl.className = 'disconnected';
}

connectWs(onSnapshot, onConnected, onDisconnected);

function animate() {
  requestAnimationFrame(animate);
  renderer.render();
}

animate();
