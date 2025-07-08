import { onMounted, onUnmounted, ref } from 'vue';

// WebSocket manager singleton
const createWebSocketManager = () => {
  let ws = null;
  let reconnectTimeout = null;
  let reconnectAttempts = 0;
  const maxReconnectAttempts = 5;
  const listeners = new Set();
  let currentUrl = null;
  let isConnected = false;

  const connect = (url) => {
    if (ws?.readyState === WebSocket.OPEN) {
      return;
    }

    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }

    try {
      currentUrl = url;
      // Fix the WebSocket URL construction
      let wsUrl;
      if (url.startsWith('http://') || url.startsWith('https://')) {
        const urlObj = new URL(url);
        const wsProtocol = urlObj.protocol === 'https:' ? 'wss:' : 'ws:';
        wsUrl = `${wsProtocol}//${urlObj.host}/ws/recording`;
      } else {
        // Handle cases where url is just host:port
        wsUrl = `ws://${url}/ws/recording`;
      }

      ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        reconnectAttempts = 0;
        isConnected = true;

        // Notify all listeners that connection is established
        for (const listener of listeners) {
          if (typeof listener === 'function') {
            try {
              listener();
            } catch (err) {
              console.error('Error in listener:', err);
            }
          }
        }
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          for (const listener of listeners) {
            if (typeof listener === 'function') {
              try {
                listener(data);
              } catch (err) {
                console.error('Error in listener:', err);
              }
            }
          }
        } catch (err) {
          console.error('Error parsing WebSocket message:', err);
        }
      };

      ws.onerror = (event) => {
        console.error('WebSocket error:', event);
        isConnected = false;
      };

      ws.onclose = (event) => {
        isConnected = false;

        for (const listener of listeners) {
          if (typeof listener === 'function') {
            try {
              listener();
            } catch (err) {
              console.error('Error in listener:', err);
            }
          }
        }

        if (listeners.size > 0 && reconnectAttempts < maxReconnectAttempts && currentUrl) {
          reconnectTimeout = setTimeout(() => {
            reconnectAttempts++;
            connect(currentUrl);
          }, 5000);
        } else if (reconnectAttempts >= maxReconnectAttempts) {
          console.error('Max reconnection attempts reached. Giving up.');
        }
      };
    } catch (err) {
      console.error('Error creating WebSocket connection:', err);
      isConnected = false;
    }
  };

  const disconnect = () => {
    if (ws) {
      ws.close();
      ws = null;
    }

    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }

    currentUrl = null;
    isConnected = false;
    reconnectAttempts = 0;
  };

  const addListener = (listener) => {
    if (typeof listener !== 'function') {
      console.error('Listener must be a function');
      return;
    }

    listeners.add(listener);
    // If we have a URL but no connection, try to connect
    if (currentUrl && (!ws || ws.readyState !== WebSocket.OPEN)) {
      connect(currentUrl);
    }
  };

  const removeListener = (listener) => {
    const wasRemoved = listeners.delete(listener);

    // Only disconnect if there are no more listeners
    if (listeners.size === 0) {
      disconnect();
    }
  };

  const getConnectionStatus = () => {
    return {
      isConnected,
      readyState: ws?.readyState,
      url: currentUrl,
      listenerCount: listeners.size,
    };
  };

  return {
    connect,
    disconnect,
    addListener,
    removeListener,
    getConnectionStatus,
  };
};

// Create a single instance of the WebSocket manager
const wsManager = createWebSocketManager();

export function useRecordingSessions(serverUrl, wsManagerInstance = wsManager) {
  const recordingSessions = ref(new Map());
  const isConnected = ref(false);
  const error = ref(null);

  const fetchInitialRecordingStatuses = async () => {
    try {
      const response = await fetch(`${serverUrl}/v1/device_manager/GetAllRecordingStatus`);
      if (!response.ok) {
        throw new Error('Failed to fetch recording statuses');
      }
      const data = await response.json();
      if (data.AllRecordingStatus) {
        for (const session of data.AllRecordingStatus) {
          recordingSessions.value.set(session.device_id, session);
        }
      }
    } catch (err) {
      console.error('Error fetching initial recording statuses:', err);
      error.value = 'Failed to fetch initial recording statuses';
    }
  };

  const isDeviceRecording = (deviceId) => {
    const session = recordingSessions.value.get(deviceId);
    return session?.is_active || false;
  };

  const getRecordingSession = (deviceId) => {
    return recordingSessions.value.get(deviceId);
  };

  onMounted(() => {
    wsManagerInstance.connect(serverUrl);
    fetchInitialRecordingStatuses();
  });

  onUnmounted(() => {
    // No need to remove listener since we're not adding one
  });

  return {
    isDeviceRecording,
    getRecordingSession,
    recordingSessions,
    isConnected,
    error,
  };
}

// Export the WebSocket manager for use in Main.vue
export { wsManager };
