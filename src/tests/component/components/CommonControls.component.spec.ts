import { mount } from '@vue/test-utils';
import { expect, it } from 'vitest';
import CommonModal from '@/components/common/CommonModal.vue';
import CommonSegmented from '@/components/common/CommonSegmented.vue';
import { setLocale } from '@/i18n';

it('keeps the labelled tabs and selects the requested view', async () => {
  setLocale('en');
  const wrapper = mount(CommonSegmented, {
    props: {
      options: [
        { value: 'console', label: 'terminalConsole' },
        { value: 'log', label: 'terminalLog' },
      ],
      label: 'Terminal',
      modelValue: 'console',
    },
  });
  expect(wrapper.attributes('aria-label')).toBe('Terminal');
  expect(wrapper.findAll('button').map((button) => button.text())).toEqual([
    'Console',
    'System log',
  ]);
  expect(wrapper.findAll('button')[0]!.attributes('aria-pressed')).toBe('true');
  await wrapper.findAll('button')[1]!.trigger('click');
  expect(wrapper.emitted('update:modelValue')).toEqual([['log']]);
  wrapper.unmount();
});

it('closes the shared modal from its button or the native cancel action', async () => {
  const wrapper = mount(CommonModal, {
    props: { modelValue: true, title: 'Terminal settings' },
    attachTo: document.body,
  });
  const button = document.querySelector<HTMLButtonElement>('[data-testid="common_modal_close"]');
  button?.click();
  expect(wrapper.emitted('update:modelValue')).toEqual([[false]]);
  await wrapper.setProps({ modelValue: false });
  await wrapper.setProps({ modelValue: true });
  document.querySelector('dialog')?.dispatchEvent(new Event('cancel', { cancelable: true }));
  expect(wrapper.emitted('update:modelValue')).toEqual([[false], [false]]);
  wrapper.unmount();
});
