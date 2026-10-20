import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import OptionsModal from '@/components/OptionsModal.vue';
import { useTerminalStore } from '@/store';
import { request } from '@/services/api';
import { setLocale } from '@/i18n';
import { settings } from '../../fixtures';

vi.mock('@/services/api', () => ({ request: vi.fn() }));

beforeEach(() => {
  setActivePinia(createPinia());
  setLocale('it');
  vi.mocked(request).mockReset();
  useTerminalStore().settings = { ...settings };
});

afterEach(() => {
  document.body.innerHTML = '';
});

function modal() {
  return mount(OptionsModal, { props: { modelValue: true }, attachTo: document.body });
}

it('offers every setting of the console, each with its label', () => {
  modal();

  for (const id of [
    'terminal_transparency',
    'terminal_text_color',
    'terminal_background_color',
    'terminal_highlight_color',
    'terminal_font_size',
    'terminal_line_height',
  ]) {
    expect(document.querySelector(`label[for="${id}"]`)?.textContent).not.toBe('');
    expect(document.getElementById(id)).not.toBeNull();
  }
});

it('edits every colour and transparency before saving, and dismisses the dialog', async () => {
  const wrapper = modal();
  for (const [id, value] of [
    ['terminal_transparency', '40'],
    ['terminal_text_color', '#112233'],
    ['terminal_background_color', '#223344'],
    ['terminal_highlight_color', '#334455'],
  ]) {
    const field = document.getElementById(id!) as HTMLInputElement;
    field.value = value!;
    field.dispatchEvent(new Event('input', { bubbles: true }));
  }
  await flushPromises();
  expect(useTerminalStore().settings?.transparency).toBe(settings.transparency);
  document.querySelector<HTMLButtonElement>('[data-testid="common_modal_close"]')?.click();
  await flushPromises();
  expect(wrapper.emitted('update:modelValue')).toEqual([[false]]);
  wrapper.unmount();
});

it('shows no settings controls until a saved appearance is available', () => {
  useTerminalStore().settings = null;
  const wrapper = modal();
  expect(document.querySelector('[data-testid="terminal_settings"]')).toBeNull();
  wrapper.unmount();
});

it('changes the console only once the settings are saved', async () => {
  const wrapper = modal();
  const terminal = useTerminalStore();
  vi.mocked(request).mockImplementation((_path, options) =>
    Promise.resolve(JSON.parse(String(options?.body))),
  );

  const bigger = document.querySelector<HTMLButtonElement>(
    '[data-testid="terminal_font_size_increase"]',
  );
  bigger?.click();
  await flushPromises();
  expect(terminal.settings?.fontSize).toBe(14);

  document.querySelector<HTMLFormElement>('[data-testid="terminal_settings"]')?.requestSubmit();
  await flushPromises();

  expect(request).toHaveBeenCalledWith('/settings/terminal', {
    method: 'PUT',
    body: JSON.stringify({ ...settings, fontSize: 15 }),
  });
  expect(terminal.settings?.fontSize).toBe(15);
  wrapper.unmount();
});

it('says why the launcher refused the settings', async () => {
  const wrapper = modal();
  vi.mocked(request).mockRejectedValue(new Error('invalid terminal settings'));

  document.querySelector<HTMLFormElement>('[data-testid="terminal_settings"]')?.requestSubmit();
  await flushPromises();

  expect(document.querySelector('[role="alert"]')?.textContent).toBe('invalid terminal settings');
  wrapper.unmount();
});
