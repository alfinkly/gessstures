const WS_URL = 'ws://localhost:3030';

export function connectWs(onSnapshot, onConnected, onDisconnected) {
  let ws = null;
  let reconnectTimeout = null;

  function connect() {
    if (ws) {
      ws.onclose = null;
      ws.close();
    }

    ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      onConnected();
      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
        reconnectTimeout = null;
      }
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.nodes && data.edges) {
          onSnapshot(data);
        }
      } catch {
      }
    };

    ws.onclose = () => {
      onDisconnected();
      scheduleReconnect();
    };

    ws.onerror = () => {};
  }

  function scheduleReconnect() {
    if (reconnectTimeout) return;
    reconnectTimeout = setTimeout(() => {
      reconnectTimeout = null;
      connect();
    }, 2000);
  }

  function send(msg) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg));
    }
  }

  connect();

  return { send };
}
