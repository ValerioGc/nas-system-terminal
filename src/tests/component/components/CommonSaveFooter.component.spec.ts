import { mount } from '@vue/test-utils';
import { beforeEach, expect, it } from 'vitest';
import CommonSaveFooter from '@/components/common/CommonSaveFooter.vue';
import { setLocale } from '@/i18n';

beforeEach(() => setLocale('en'));

it('submits the form it sits in, and is disabled while saving', async () => {
  const wrapper = mount(CommonSaveFooter, { props: { busy: false, testid: 'save' } });
  const button = wrapper.get('[data-testid="save"]');

  expect(button.attributes('type')).toBe('submit');
  expect(button.text()).toBe('Save');
  expect(button.attributes('disabled')).toBeUndefined();

  await wrapper.setProps({ busy: true });
  expect(button.attributes('disabled')).toBeDefined();
});
