import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { expect, it } from 'vitest';
import OutputPanel from '@/components/OutputPanel.vue';
import type { TerminalEntry } from '@/types';

function panel(lines: TerminalEntry[]) {
  return mount(OutputPanel, {
    props: { lines, received: lines.length, waiting: 'Nothing yet.', label: 'Log', testid: 'out' },
  });
}

it('follows new output only while the reader is at the end', async () => {
  const wrapper = panel([]);
  const output = wrapper.get('[role="log"]').element;
  Object.defineProperties(output, {
    scrollHeight: { value: 300, configurable: true },
    clientHeight: { value: 100 },
  });
  output.scrollTop = 200;
  await wrapper.setProps({ received: 1 });
  await nextTick();
  expect(output.scrollTop).toBe(300);
  output.scrollTop = 0;
  await wrapper.setProps({ received: 2 });
  await nextTick();
  expect(output.scrollTop).toBe(0);
  wrapper.unmount();
});

it('says what it waits for until there is a line, and names itself', () => {
  const wrapper = panel([]);

  expect(wrapper.text()).toBe('Nothing yet.');
  expect(wrapper.get('[role="log"]').attributes('aria-label')).toBe('Log');
  expect(wrapper.find('[data-testid="out"]').exists()).toBe(true);
});

it('shows every line as text, including markup, without executing it', () => {
  const wrapper = panel([
    { text: '/ $ cat page.html' },
    { text: '<img src=x onerror=alert(1)>' },
    { text: 'Exited with code 1.' },
  ]);
  const lines = wrapper.findAll('.terminal_line');

  expect(wrapper.find('img').exists()).toBe(false);
  expect(lines.map((line) => line.text())).toEqual([
    '/ $ cat page.html',
    '<img src=x onerror=alert(1)>',
    'Exited with code 1.',
  ]);

  expect(wrapper.findAll('.terminal_output')).toHaveLength(1);
});
