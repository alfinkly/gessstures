const WS_URL = 'ws://localhost:3030';
const W = 640, H = 480;
const COLORS = ['#6366f1', '#f59e0b', '#10b981', '#ef4444'];

const video = document.getElementById('video');
const overlay = document.getElementById('overlay-canvas');
const personCount = document.getElementById('person-count');
const wsIndicator = document.getElementById('ws-indicator');
const tilesRow = document.getElementById('tiles-row');
const loading = document.getElementById('loading');
const container = document.getElementById('container');

overlay.width = W;
overlay.height = H;
const ovCtx = overlay.getContext('2d');

const captureCanvas = document.createElement('canvas');
captureCanvas.width = W;
captureCanvas.height = H;
const capCtx = captureCanvas.getContext('2d');

let detectionResult = null;
let wsConnected = false;
let tiles = [];

// ── Camera ──────────────────────────────────────────────────────

async function startCamera() {
  const stream = await navigator.mediaDevices.getUserMedia({
    video: { width: { exact: W }, height: { exact: H }, facingMode: 'user' },
    audio: false,
  });
  video.srcObject = stream;
  await video.play();
}

// ── WebSocket ───────────────────────────────────────────────────

let ws = null;

function connectWs() {
  ws = new WebSocket(WS_URL);
  ws.binaryType = 'arraybuffer';

  ws.onopen = () => {
    wsConnected = true;
    wsIndicator.textContent = '● online';
    wsIndicator.className = 'on';
    loading.classList.add('hidden');
    container.classList.remove('hidden');
  };

  ws.onmessage = (e) => {
    if (typeof e.data !== 'string') return;
    try {
      const data = JSON.parse(e.data);
      if (data.person_count !== undefined) {
        detectionResult = data;
      }
    } catch {}
  };

  ws.onclose = () => {
    wsConnected = false;
    wsIndicator.textContent = '● offline';
    wsIndicator.className = 'off';
    setTimeout(connectWs, 2000);
  };

  ws.onerror = () => {};
}

// ── Frame sender (runs at ~8fps, sends JPEG over WS) ──────────

let sending = false;

async function sendFrame() {
  if (!wsConnected || sending) return;
  sending = true;

  capCtx.drawImage(video, 0, 0, W, H);
  const blob = await new Promise((r) => captureCanvas.toBlob(r, 'image/jpeg', 0.8));
  if (ws.readyState === WebSocket.OPEN) {
    ws.send(await blob.arrayBuffer());
  }
  sending = false;
}

// ── Tile management ────────────────────────────────────────────

function updateTiles(count) {
  while (tiles.length < count) {
    const i = tiles.length;
    const wrap = document.createElement('div');
    wrap.className = 'tile-wrapper';
    wrap.style.aspectRatio = 'auto';
    const num = document.createElement('div');
    num.className = 'tile-num';
    num.textContent = `#${i + 1}`;
    const c = document.createElement('canvas');
    wrap.append(c, num);
    tilesRow.appendChild(wrap);
    tiles.push({ wrap, canvas: c, ctx: c.getContext('2d') });
  }
  while (tiles.length > count) {
    const t = tiles.pop();
    t.wrap.remove();
  }
}

// ── Render loop ────────────────────────────────────────────────

function render() {
  requestAnimationFrame(render);

  // 1. Draw video at fixed W×H on overlay (source of truth for coordinates)
  ovCtx.drawImage(video, 0, 0, W, H);

  const result = detectionResult;
  let count = 0;

  if (result && result.persons) {
    count = result.persons.length;

    // 2. Tiles — crop from overlay canvas (NOT video, avoids native-size mismatch)
    personCount.textContent = count;
    updateTiles(count);

    for (let i = 0; i < count; i++) {
      const p = result.persons[i];
      const [cx, cy, w, h] = p.bbox;
      const color = COLORS[i % COLORS.length];
      const t = tiles[i];

      // Source rect in overlay pixels (clamped to [0,W]×[0,H])
      let sx = (cx - w / 2) * W;
      let sy = (cy - h / 2) * H;
      let sw = w * W;
      let sh = h * H;

      if (sx < 0) { sw += sx; sx = 0; }
      if (sy < 0) { sh += sy; sy = 0; }
      if (sx + sw > W) sw = W - sx;
      if (sy + sh > H) sh = H - sy;

      if (sw < 4 || sh < 4) continue;

      // Tile canvas matches the crop's aspect ratio (no stretch)
      const tileW = 240;
      const tileH = Math.max(80, Math.round(tileW * (sh / sw)));
      t.canvas.width = tileW;
      t.canvas.height = tileH;
      t.wrap.style.height = tileH + 'px';

      t.ctx.drawImage(overlay, sx, sy, sw, sh, 0, 0, tileW, tileH);

      // Keypoints on tile (adjusted for clamped crop)
      if (p.keypoints) {
        const leftNorm = sx / W;      // left edge of visible crop (normalized)
        const topNorm = sy / H;       // top edge of visible crop
        const visW = sw / W;          // visible width (normalized)
        const visH = sh / H;          // visible height
        for (const kp of p.keypoints) {
          const kx = ((kp[0] - leftNorm) / visW) * tileW;
          const ky = ((kp[1] - topNorm) / visH) * tileH;
          if (kx < 0 || kx > tileW || ky < 0 || ky > tileH) continue;
          t.ctx.beginPath();
          t.ctx.arc(kx, ky, 3, 0, Math.PI * 2);
          t.ctx.fillStyle = color;
          t.ctx.fill();
        }
      }
    }

    // 3. Bboxes and keypoints on overlay (can be out-of-bounds, strokeRect handles it)
    for (let i = 0; i < count; i++) {
      const p = result.persons[i];
      const [cx, cy, w, h] = p.bbox;
      const color = COLORS[i % COLORS.length];
      const bx = (cx - w / 2) * W;
      const by = (cy - h / 2) * H;
      const bw = w * W;
      const bh = h * H;

      ovCtx.strokeStyle = color;
      ovCtx.lineWidth = 2;
      ovCtx.strokeRect(bx, by, bw, bh);

      if (p.keypoints) {
        for (const kp of p.keypoints) {
          ovCtx.beginPath();
          ovCtx.arc(kp[0] * W, kp[1] * H, 3, 0, Math.PI * 2);
          ovCtx.fillStyle = color;
          ovCtx.fill();
        }
      }
    }
  } else {
    personCount.textContent = '0';
    updateTiles(0);
  }
}

// ── Init ──────────────────────────────────────────────────────

async function init() {
  loading.classList.remove('hidden');
  await startCamera();
  connectWs();
  setInterval(sendFrame, 120); // ~8fps JPEG over WebSocket
  render();
}

init().catch((e) => {
  loading.textContent = `Error: ${e.message}`;
});
