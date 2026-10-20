import { expect, test } from '@playwright/test';
import { settings } from '../fixtures';

test.beforeEach(async ({ page }) => {
  let appearance = { ...settings };
  await page.route('**/api/settings/terminal', async (route) => {
    if (route.request().method() === 'PUT') {
      appearance = route.request().postDataJSON();
    }

    await route.fulfill({ json: appearance });
  });
  await page.routeWebSocket('**/api/terminal/session', (socket) => {
    socket.send(Buffer.from('nas$ '));
    socket.onMessage((data) => {
      if (typeof data === 'string') {
        return;
      }
      const input = data.toString();
      socket.send(data);
      if (input === '\r') {
        socket.send(Buffer.from('\r\n/tmp\r\nnas$ '));
      }
      if (input === '\x03') {
        socket.send(Buffer.from('^C\r\nnas$ '));
      }
    });
  });
  await page.addInitScript(() => {
    class Source {
      static CLOSED = 2;
      readyState = 0;
      onopen: (() => void) | null = null;
      onmessage: ((event: MessageEvent<string>) => void) | null = null;
      constructor() {
        queueMicrotask(() => {
          if (this.readyState === 2) {
            return;
          }
          this.readyState = 1;
          this.onopen?.();
          this.onmessage?.(new MessageEvent('message', { data: 'Started nas-os.service.' }));
        });
      }
      close() {
        this.readyState = 2;
      }
    }
    Object.defineProperty(window, 'EventSource', { value: Source });
  });
});

test('renders the interactive console, sends keys and keeps the shell across tabs', async ({
  page,
}) => {
  await page.goto('/?lang=en');
  const consolePanel = page.getByTestId('terminal_output');
  const input = consolePanel.locator('.xterm-helper-textarea');
  await expect(page.getByTestId('terminal_connection')).toHaveAttribute('title', 'Connected');
  await input.focus();
  await input.pressSequentially('pwd');
  await input.press('Enter');
  await expect(consolePanel).toContainText('/tmp');
  await input.press('Control+c');
  await expect(consolePanel).toContainText('^C');
  await page.getByTestId('terminal_view_log').click();
  await expect(page.getByTestId('terminal_log')).toContainText('Started nas-os.service.');
  await expect(consolePanel).toBeHidden();
  await page.getByTestId('terminal_view_console').click();
  await expect(consolePanel).toBeVisible();
  await expect(consolePanel).toContainText('/tmp');
  await expect(input).toBeFocused();
  await page.setViewportSize({ width: 1100, height: 720 });
  await expect
    .poll(() => consolePanel.locator('.xterm-screen').evaluate((element) => element.clientWidth))
    .toBeGreaterThan(500);
  await page.getByTestId('terminal_clear').click();
  await expect(consolePanel).not.toContainText('/tmp');
});

test('keeps decimal appearance settings and closes the shared modal with Escape', async ({
  page,
}) => {
  await page.goto('/?lang=en');
  await page.getByTestId('terminal_settings_open').click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.getByTestId('terminal_line_height_increase').click();
  await expect(page.getByTestId('terminal_line_height')).toHaveValue('1.6');
  await page.getByTestId('terminal_transparency').focus();
  await page.getByTestId('terminal_transparency').press('End');
  const saved = page.waitForRequest(
    (request) => request.method() === 'PUT' && request.url().endsWith('/api/settings/terminal'),
  );
  await page.getByTestId('terminal_settings_save').click();
  expect((await saved).postDataJSON()).toMatchObject({ lineHeight: 1.6, transparency: 100 });
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByTestId('terminal_app')).toHaveCSS('--terminal-line-height', '1.6');
  await expect(page.locator('body')).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
});
