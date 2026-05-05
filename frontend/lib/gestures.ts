export type Gesture =
  | 'none'
  | 'waiting'
  | 'raising_right'
  | 'raising_left'
  | 'waving_right'
  | 'waving_left'
  | 'call_me_right'
  | 'call_me_left'

export interface GestureInfo {
  gesture: Gesture
  label: string
  color: string
}

const GESTURE_LABELS: Record<Gesture, string> = {
  none: '—',
  waiting: 'waiting',
  raising_right: '✋ правая',
  raising_left: '✋ левая',
  waving_right: '👋 машу правой',
  waving_left: '👋 машу левой',
  call_me_right: '🤙 созвонимся',
  call_me_left: '🤙 созвонимся',
}

const GESTURE_COLORS: Record<Gesture, string> = {
  none: '#888',
  waiting: '#888',
  raising_right: '#f59e0b',
  raising_left: '#f59e0b',
  waving_right: '#10b981',
  waving_left: '#10b981',
  call_me_right: '#6366f1',
  call_me_left: '#6366f1',
}

export function gestureInfo(g: Gesture): GestureInfo {
  return { gesture: g, label: GESTURE_LABELS[g], color: GESTURE_COLORS[g] }
}

function dist(a: number[], b: number[]) {
  const dx = a[0] - b[0], dy = a[1] - b[1]
  return Math.sqrt(dx * dx + dy * dy)
}

export function detectGesture(
  kp: number[][],
  rWristHistory: { x: number; y: number }[],
  lWristHistory: { x: number; y: number }[],
): Gesture {
  if (kp.length < 23) return 'none'

  const lWrist = kp[15]
  const rWrist = kp[16]
  const lPinky = kp[17]
  const rPinky = kp[18]
  const lIdx = kp[19]
  const rIdx = kp[20]
  const lThumb = kp[21]
  const rThumb = kp[22]

  const shoulderY = (kp[11][1] + kp[12][1]) / 2
  const rRaised = rWrist[1] < shoulderY + 0.08
  const lRaised = lWrist[1] < shoulderY + 0.08

  // call me: thumb + pinky extended, index curled
  const rCall =
    rRaised &&
    dist(rThumb, rWrist) > 0.15 &&
    dist(rPinky, rWrist) > 0.15 &&
    dist(rIdx, rWrist) < 0.12
  const lCall =
    lRaised &&
    dist(lThumb, lWrist) > 0.15 &&
    dist(lPinky, lWrist) > 0.15 &&
    dist(lIdx, lWrist) < 0.12

  if (rCall) return 'call_me_right'
  if (lCall) return 'call_me_left'

  // waving: raised hand + oscillation
  const waveCheck = (history: { x: number }[]) => {
    if (history.length < 12) return false
    const last = history.slice(-12)
    let changes = 0
    for (let i = 2; i < last.length; i++) {
      const prev = last[i - 1].x - last[i - 2].x
      const curr = last[i].x - last[i - 1].x
      if (prev * curr < 0 && Math.abs(prev) > 0.015 && Math.abs(curr) > 0.015) {
        changes++
      }
    }
    return changes >= 3
  }

  if (rRaised && waveCheck(rWristHistory)) return 'waving_right'
  if (lRaised && waveCheck(lWristHistory)) return 'waving_left'

  if (rRaised) return 'raising_right'
  if (lRaised) return 'raising_left'

  return 'none'
}

export function shouldShowWaiting(
  current: Gesture,
  lastActiveTime: number,
  now: number,
): boolean {
  return current === 'none' && lastActiveTime > 0 && (now - lastActiveTime) > 5000
}
