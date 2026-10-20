import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MAX_LINES, useTerminalStore } from '@/store';
import { request } from '@/services/api';
import { setLocale } from '@/i18n';
import { FakeEventSource, FakeWebSocket, settings } from '../fixtures';

vi.mock('@/services/api', () => ({ request: vi.fn() }));

beforeEach(() => {
  setActivePinia(createPinia());
  setLocale('it');
  vi.stubGlobal('EventSource', FakeEventSource);
  vi.stubGlobal('WebSocket', FakeWebSocket);
  vi.mocked(request).mockReset().mockResolvedValue(undefined);
});

afterEach(() => vi.unstubAllGlobals());

describe('interactive terminal store', () => {
  it('opens a websocket within the app and sends the latest size on connection', () => {
    const terminal = useTerminalStore();
    terminal.resize(132, 42);
    terminal.connect();
    const socket = FakeWebSocket.last;
    expect(socket.url.pathname).toBe('/api/terminal/session');
    expect(socket.url.protocol).toBe('ws:');
    expect(socket.binaryType).toBe('arraybuffer');
    expect(terminal.connection).toBe('connecting');
    socket.open();
    expect(terminal.connection).toBe('open');
    expect(socket.sent).toEqual([JSON.stringify({ cols: 132, rows: 42 })]);
    terminal.resize(80, 24);
    expect(socket.sent.at(-1)).toBe('{"cols":80,"rows":24}');
  });

  it('uses wss on HTTPS without losing the app prefix', () => {
    vi.stubGlobal('window', { location: { href: 'https://nas.local/apps/nas-system-terminal/' } });
    useTerminalStore().connect();
    expect(FakeWebSocket.last.url.href).toBe(
      'wss://nas.local/apps/nas-system-terminal/api/terminal/session',
    );
  });

  it('transports control characters and output unchanged without recording passwords', () => {
    const terminal = useTerminalStore();
    const output = vi.fn();
    terminal.setOutput(output);
    terminal.send(new Uint8Array([3]));
    terminal.connect();
    const socket = FakeWebSocket.last;
    socket.open();
    const bytes = new TextEncoder().encode('password\r\x03\t');
    terminal.send(bytes);
    expect(socket.sent.at(-1)).toEqual(bytes);
    terminal.send(new Uint8Array(65537));
    expect(socket.sent.at(-2) as Uint8Array).toHaveLength(65536);
    expect(socket.sent.at(-1) as Uint8Array).toHaveLength(1);
    socket.print('\x1b[31mIt’s ready\r\n');
    expect(new TextDecoder().decode(output.mock.calls[0]![0])).toBe('\x1b[31mIt’s ready\r\n');
    expect(terminal.received).toBe(1);
    terminal.setOutput(null);
    socket.print('without listener');
    terminal.clear();
    expect(terminal.received).toBe(0);
    expect(terminal.cleared).toBe(1);
  });

  it('closes the previous shell and ignores its late close event', () => {
    const terminal = useTerminalStore();
    terminal.connect();
    const previous = FakeWebSocket.last;
    previous.open();
    terminal.connect();
    expect(previous.closed).toBe(true);
    previous.onclose?.();
    expect(terminal.connection).toBe('connecting');
    FakeWebSocket.last.onerror?.();
    expect(terminal.connection).toBe('closed');
    terminal.disconnect();
    expect(FakeWebSocket.last.closed).toBe(true);
    terminal.send(new Uint8Array([3]));
  });

  it('reads and saves appearance and reports load/save failures', async () => {
    const terminal = useTerminalStore();
    vi.mocked(request).mockResolvedValueOnce(settings);
    await terminal.loadSettings();
    expect(terminal.settings).toEqual(settings);
    const bigger = { ...settings, fontSize: 18 };
    vi.mocked(request).mockResolvedValueOnce(bigger);
    await terminal.saveSettings(bigger);
    expect(terminal.settings).toEqual(bigger);
    vi.mocked(request).mockRejectedValueOnce(new Error('invalid settings'));
    await terminal.saveSettings({ ...settings, fontSize: 20 });
    expect(terminal.settings).toEqual(bigger);
    expect(terminal.error).toBe('invalid settings');
    expect(terminal.saving).toBe(false);
    vi.mocked(request).mockRejectedValueOnce(new Error('offline'));
    await terminal.loadSettings();
    expect(terminal.error).toBe('offline');
  });

  it('keeps the journal bounded, follows its connection and closes the stream', () => {
    const terminal = useTerminalStore();
    terminal.followLog();
    const log = FakeEventSource.last!;
    expect(log.url).toBe('api/terminal/log');
    log.connect();
    expect(terminal.logConnection).toBe('open');
    log.print(...Array.from({ length: MAX_LINES + 2 }, (_, i) => `entry ${i}`));
    expect(terminal.log).toHaveLength(MAX_LINES);
    expect(terminal.log[0]?.text).toBe('entry 2');
    log.fail(false);
    expect(terminal.logConnection).toBe('connecting');
    log.fail(true);
    expect(terminal.logConnection).toBe('closed');
    terminal.followLog();
    expect(log.closed).toBe(true);
    terminal.clearLog();
    terminal.stopLog();
    expect(terminal.log).toEqual([]);
    expect(FakeEventSource.last?.closed).toBe(true);
  });
});
