import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import App from '@/App.vue';
import { request } from '@/services/api';
import { useTerminalStore } from '@/store';
import { setLocale } from '@/i18n';
import { FakeEventSource, FakeWebSocket, FakeTerminal, settings } from '../fixtures';

vi.mock('@/services/api', () => ({ request: vi.fn() }));
beforeEach(() => {
  setActivePinia(createPinia());
  setLocale('en');
  vi.stubGlobal('EventSource', FakeEventSource);
  vi.stubGlobal('WebSocket', FakeWebSocket);
  FakeEventSource.opened.clear();
  vi.mocked(request).mockReset().mockResolvedValue(settings);
});
afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

it('mounts the terminal inside the standard launcher context', async () => {
  const wrapper = mount(App);
  await flushPromises();
  expect(document.documentElement.dataset.theme).toBe('light');
  expect(wrapper.get('section').attributes('aria-label')).toBe('Terminal');
  wrapper.unmount();
});

it('opens an interactive shell, transports typed data and closes everything on unmount', async () => {
  const wrapper = mount(App, { attachTo: document.body });
  await flushPromises();
  const socket = FakeWebSocket.last;
  socket.open();
  await flushPromises();
  expect(wrapper.get('[data-testid="terminal_connection"]').attributes('title')).toBe('Connected');
  expect(document.activeElement).toBe(FakeTerminal.last.field);
  FakeTerminal.last.data?.('ls\r');
  expect(new TextDecoder().decode(socket.sent.at(-1) as Uint8Array)).toBe('ls\r');
  FakeTerminal.last.binary?.('\xff');
  expect(socket.sent.at(-1)).toEqual(new Uint8Array([255]));
  socket.print('hello');
  await flushPromises();
  expect(wrapper.get('[data-testid="terminal_output"]').text()).toContain('hello');
  wrapper.unmount();
  expect(socket.closed).toBe(true);
  expect(FakeTerminal.last.disposed).toBe(true);
});

it('keeps appearance, clearing, reconnection and settings controls', async () => {
  const wrapper = mount(App, { attachTo: document.body });
  await flushPromises();
  expect(wrapper.get('[data-testid="terminal_app"]').attributes('style')).toContain(
    '--terminal-font-size: 14px',
  );
  expect(wrapper.get('section').attributes('style')).toContain(
    'color-mix(in srgb, var(--surface) 80%, transparent)',
  );
  expect(FakeTerminal.last.options.theme).toMatchObject({
    foreground: settings.textColor,
    background: '#0e1922cc',
  });
  FakeWebSocket.last.open();
  FakeWebSocket.last.print('old output');
  await flushPromises();
  await wrapper.get('[data-testid="terminal_clear"]').trigger('click');
  expect(wrapper.get('[data-testid="terminal_output"]').text()).not.toContain('old output');
  FakeWebSocket.last.close();
  await flushPromises();
  await wrapper.get('[data-testid="terminal_reconnect"]').trigger('click');
  expect(useTerminalStore().connection).toBe('connecting');
  await wrapper.get('[data-testid="terminal_settings_open"]').trigger('click');
  expect(document.querySelector('[data-testid="terminal_settings"]')).not.toBeNull();
  wrapper.unmount();
});

it('switches to the journal without replacing the shell and reconnects/clears the journal', async () => {
  const wrapper = mount(App, { attachTo: document.body });
  await flushPromises();
  const socket = FakeWebSocket.last;
  await wrapper.get('[data-testid="terminal_view_log"]').trigger('click');
  const log = FakeEventSource.last!;
  log.connect();
  log.print('Started nas-os.service.');
  await flushPromises();
  expect(wrapper.get('[data-testid="terminal_log"]').text()).toContain('Started nas-os.service.');
  expect(wrapper.get('[data-testid="terminal_output"]').isVisible()).toBe(false);
  expect(socket.closed).toBe(false);
  await wrapper.get('[data-testid="terminal_clear"]').trigger('click');
  expect(useTerminalStore().log).toEqual([]);
  log.fail(true);
  await flushPromises();
  await wrapper.get('[data-testid="terminal_reconnect"]').trigger('click');
  expect(FakeEventSource.last).not.toBe(log);
  await wrapper.get('[data-testid="terminal_view_console"]').trigger('click');
  expect(FakeEventSource.last?.closed).toBe(true);
  expect(wrapper.get('[data-testid="terminal_output"]').isVisible()).toBe(true);
  wrapper.unmount();
});

it('shows settings failures outside the modal', async () => {
  vi.mocked(request).mockRejectedValueOnce(new Error('offline'));
  const wrapper = mount(App);
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toBe('offline');
  wrapper.unmount();
});
