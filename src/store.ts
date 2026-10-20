import { defineStore } from 'pinia';
import { ref } from 'vue';
import { request } from '@/services/api';
import { failureMessage } from '@/services/failureMessage';
import type { TerminalConnection, TerminalEntry, TerminalSettings } from '@/types';

const SESSION_URL = 'api/terminal/session';
const LOG_URL = 'api/terminal/log';
const SETTINGS_PATH = '/settings/terminal';

export const MAX_LINES = 2000;

function append(lines: TerminalEntry[], entry: TerminalEntry) {
  lines.push(entry);
  if (lines.length > MAX_LINES) {
    lines.splice(0, lines.length - MAX_LINES);
  }
}

export const useTerminalStore = defineStore('nas-system-terminal', () => {
  const received = ref(0);
  const cleared = ref(0);
  const connection = ref<TerminalConnection>('closed');
  const log = ref<TerminalEntry[]>([]);
  const logReceived = ref(0);
  const logConnection = ref<TerminalConnection>('closed');
  const settings = ref<TerminalSettings | null>(null);
  const saving = ref(false);
  const error = ref('');
  let socket: WebSocket | null = null;
  let output: ((data: Uint8Array) => void) | null = null;
  let size = { cols: 80, rows: 24 };
  let logSource: EventSource | null = null;

  function followLog() {
    stopLog();
    logConnection.value = 'connecting';
    const stream = new EventSource(LOG_URL);
    stream.onopen = () => {
      logConnection.value = 'open';
    };
    stream.onmessage = (event: MessageEvent<string>) => {
      append(log.value, { text: event.data });
      logReceived.value += 1;
    };
    stream.onerror = () => {
      logConnection.value = stream.readyState === EventSource.CLOSED ? 'closed' : 'connecting';
    };
    logSource = stream;
  }

  function stopLog() {
    logSource?.close();
    logSource = null;
    logConnection.value = 'closed';
  }

  function clearLog() {
    log.value = [];
  }

  function setOutput(handler: ((data: Uint8Array) => void) | null) {
    output = handler;
  }

  function connect() {
    disconnect();
    connection.value = 'connecting';
    const url = new URL(SESSION_URL, window.location.href);
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    const stream = new WebSocket(url);
    stream.binaryType = 'arraybuffer';
    socket = stream;
    stream.onopen = () => {
      connection.value = 'open';
      resize(size.cols, size.rows);
    };
    stream.onmessage = (event: MessageEvent<ArrayBuffer>) => {
      received.value += 1;
      output?.(new Uint8Array(event.data));
    };
    stream.onerror = stream.onclose = () => {
      if (socket === stream) {
        connection.value = 'closed';
      }
    };
  }

  function disconnect() {
    const previous = socket;
    socket = null;
    previous?.close();
    connection.value = 'closed';
  }

  function send(data: Uint8Array<ArrayBuffer>) {
    if (socket?.readyState === WebSocket.OPEN) {
      for (let offset = 0; offset < data.length; offset += 65536) {
        socket.send(data.subarray(offset, offset + 65536));
      }
    }
  }

  function resize(cols: number, rows: number) {
    size = { cols, rows };
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(size));
    }
  }

  function clear() {
    cleared.value += 1;
    received.value = 0;
  }

  async function loadSettings() {
    try {
      settings.value = await request<TerminalSettings>(SETTINGS_PATH);
    } catch (reason) {
      error.value = failureMessage(reason);
    }
  }

  async function saveSettings(next: TerminalSettings) {
    saving.value = true;
    error.value = '';
    try {
      settings.value = await request<TerminalSettings>(SETTINGS_PATH, {
        method: 'PUT',
        body: JSON.stringify(next),
      });
    } catch (reason) {
      error.value = failureMessage(reason);
    } finally {
      saving.value = false;
    }
  }

  return {
    received,
    connection,
    cleared,
    log,
    logReceived,
    logConnection,
    settings,
    saving,
    error,
    connect,
    disconnect,
    setOutput,
    send,
    resize,
    clear,
    followLog,
    stopLog,
    clearLog,
    loadSettings,
    saveSettings,
  };
});
