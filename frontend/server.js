const { createServer } = require('http')
const next = require('next')
const { WebSocket: WsClient } = require('ws')

const dev = process.env.NODE_ENV !== 'production'
const app = next({ dev, hostname: '0.0.0.0', port: parseInt(process.env.PORT || '3000', 10) })
const handle = app.getRequestHandler()

const WS_BRIDGE = process.env.WS_BRIDGE || 'ws://localhost:3030'

app.prepare().then(() => {
  const server = createServer((req, res) => {
    handle(req, res)
  })

  server.on('upgrade', (req, socket, head) => {
    if (req.url && req.url.startsWith('/ws')) {
      const target = `${WS_BRIDGE}${req.url.replace('/ws', '')}`
      const proxy = new WsClient(target)
      proxy.on('open', () => {
        const resHeaders = [
          'HTTP/1.1 101 Switching Protocols',
          'Upgrade: websocket',
          'Connection: Upgrade',
        ]
        socket.write(resHeaders.join('\r\n') + '\r\n\r\n')
        proxy._socket.pipe(socket)
        socket.pipe(proxy._socket)
      })
      proxy.on('error', () => socket.destroy())
      socket.on('error', () => proxy.close())
    } else {
      socket.destroy()
    }
  })

  const port = parseInt(process.env.PORT || '3000', 10)
  server.listen(port, '0.0.0.0', () => {
    console.log(`> Next.js + WS proxy on http://0.0.0.0:${port}`)
    console.log(`> WS bridge target: ${WS_BRIDGE}`)
  })
})
