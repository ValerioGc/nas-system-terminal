Object.defineProperties(HTMLDialogElement.prototype, {
  showModal: {
    configurable: true,
    writable: true,
    value(this: HTMLDialogElement) {
      this.open = true;
    },
  },
  close: {
    configurable: true,
    writable: true,
    value(this: HTMLDialogElement) {
      this.open = false;
    },
  },
});

import { beforeEach, vi } from 'vitest';
import { FakeFitAddon, FakeTerminal } from '../fixtures';
vi.mock('@xterm/xterm', () => ({ Terminal: FakeTerminal }));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: FakeFitAddon }));

beforeEach(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
});
