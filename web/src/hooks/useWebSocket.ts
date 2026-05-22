import { useEffect, useRef, useState, useCallback } from 'react';
import { useAuthStore } from '../stores/auth';

export interface PipelineEvent {
  project_id: string;
  step_name: string;
  status: string;
  progress: number;
  timestamp: string;
}

export function useWebSocket(projectId?: string) {
  const [events, setEvents] = useState<PipelineEvent[]>([]);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout>>();
  const token = useAuthStore((s) => s.token);

  const connect = useCallback(() => {
    if (!token) return;

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    let url = `${protocol}//${host}/api/v1/ws?token=${token}`;
    if (projectId) url += `&project_id=${projectId}`;

    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => setConnected(true);
    ws.onclose = () => {
      setConnected(false);
      reconnectTimer.current = setTimeout(connect, 3000);
    };
    ws.onerror = () => ws.close();
    ws.onmessage = (e) => {
      try {
        const event: PipelineEvent = JSON.parse(e.data);
        setEvents((prev) => [...prev.slice(-99), event]);
      } catch {}
    };
  }, [token, projectId]);

  useEffect(() => {
    connect();
    return () => {
      clearTimeout(reconnectTimer.current);
      wsRef.current?.close();
    };
  }, [connect]);

  const clearEvents = useCallback(() => setEvents([]), []);

  return { events, connected, clearEvents };
}
