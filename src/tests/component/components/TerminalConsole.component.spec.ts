import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, expect, it, vi } from 'vitest';
import TerminalConsole from '@/components/TerminalConsole.vue';
import { useTerminalStore } from '@/store';
import { FakeFitAddon, FakeTerminal, FakeWebSocket, settings } from '../../fixtures';

afterEach(() => vi.unstubAllGlobals());

it('fits visible terminal dimensions and reapplies appearance when settings change', async () => {
  setActivePinia(createPinia());
  vi.stubGlobal('WebSocket', FakeWebSocket);
  let resize: (() => void) | undefined;
  const disconnect = vi.fn();
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(callback: () => void) {
        resize = callback;
      }
      observe() {}
      disconnect = disconnect;
    },
  );
  const store = useTerminalStore();
  const wrapper = mount(TerminalConsole, { attachTo: document.body });
  Object.defineProperties(wrapper.element, {
    clientWidth: { value: 800 },
    clientHeight: { value: 500 },
  });
  resize?.();
  expect(FakeFitAddon.last.fitted).toBe(1);
  store.settings = { ...settings, fontSize: 20, transparency: 100 };
  await flushPromises();
  expect(FakeTerminal.last.options).toMatchObject({
    fontSize: 20,
    lineHeight: 1.5,
    theme: { background: '#0e192200' },
  });
  FakeWebSocket.last.open();
  await flushPromises();
  expect(FakeWebSocket.last.sent.at(-1)).toBe('{"cols":80,"rows":24}');
  wrapper.unmount();
  expect(disconnect).toHaveBeenCalledOnce();
});
