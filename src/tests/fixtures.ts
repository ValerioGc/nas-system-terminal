import type { TerminalSettings } from '@/types';

export const settings: TerminalSettings = {
  transparency: 20,
  textColor: '#d6e2ea',
  backgroundColor: '#0e1922',
  highlightColor: '#3fd39b',
  fontSize: 14,
  lineHeight: 1.5,
};

type Listener = (event: MessageEvent<string>) => void;

export class FakeEventSource {
  static readonly CLOSED = 2;
  static last: FakeEventSource | null = null;
  static readonly opened = new Map<string, FakeEventSource>();

  readyState = 0;
  closed = false;
  onopen: (() => void) | null = null;
  onmessage: Listener | null = null;
  onerror: (() => void) | null = null;

  constructor(readonly url: string) {
    FakeEventSource.last = this;
    FakeEventSource.opened.set(url, this);
  }

  close() {
    this.closed = true;
    this.readyState = FakeEventSource.CLOSED;
  }

  connect() {
    this.readyState = 1;
    this.onopen?.();
  }

  print(...lines: string[]) {
    for (const data of lines) {
      this.onmessage?.({ data } as MessageEvent<string>);
    }
  }

  fail(closed: boolean) {
    this.readyState = closed ? FakeEventSource.CLOSED : 0;
    this.onerror?.();
  }
}

export class FakeWebSocket {
  static readonly OPEN = 1;
  static last: FakeWebSocket;
  readyState = 0;
  binaryType = '';
  closed = false;
  sent: (Uint8Array | string)[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((event: MessageEvent<ArrayBuffer>) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  constructor(readonly url: URL) {
    FakeWebSocket.last = this;
  }
  send(data: Uint8Array | string) {
    this.sent.push(data);
  }
  close() {
    this.closed = true;
    this.readyState = 3;
    this.onclose?.();
  }
  open() {
    this.readyState = 1;
    this.onopen?.();
  }
  print(text: string) {
    this.onmessage?.({ data: new TextEncoder().encode(text).buffer } as MessageEvent<ArrayBuffer>);
  }
}

export class FakeTerminal {
  static last: FakeTerminal;
  cols = 80;
  rows = 24;
  disposed = false;
  options: Record<string, unknown>;
  panel?: HTMLElement;
  field?: HTMLTextAreaElement;
  data?: (data: string) => void;
  binary?: (data: string) => void;
  constructor(options: Record<string, unknown>) {
    this.options = options;
    FakeTerminal.last = this;
  }
  loadAddon() {}
  open(panel: HTMLElement) {
    this.panel = panel;
    this.field = document.createElement('textarea');
    panel.append(this.field);
  }
  onData(handler: (data: string) => void) {
    this.data = handler;
  }
  onBinary(handler: (data: string) => void) {
    this.binary = handler;
  }
  write(data: Uint8Array) {
    this.panel?.append(document.createTextNode(new TextDecoder().decode(data)));
  }
  clear() {
    this.panel?.querySelectorAll('p').forEach((node) => node.remove());
    if (this.panel) {
      for (const node of [...this.panel.childNodes]) {
        if (node.nodeType === Node.TEXT_NODE) {
          node.remove();
        }
      }
    }
  }
  focus() {
    this.field?.focus();
  }
  dispose() {
    this.disposed = true;
  }
}

export class FakeFitAddon {
  static last: FakeFitAddon;
  fitted = 0;
  constructor() {
    FakeFitAddon.last = this;
  }
  fit() {
    this.fitted += 1;
  }
}
