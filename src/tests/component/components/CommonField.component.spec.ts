import { mount } from '@vue/test-utils';
import { expect, it } from 'vitest';
import CommonField from '@/components/common/CommonField.vue';

it('holds the label and the control it is given, in that order', () => {
  const wrapper = mount(CommonField, {
    slots: { default: '<label for="name">Name</label><input id="name" />' },
  });

  expect(wrapper.get('label').attributes('for')).toBe('name');
  expect(wrapper.get('label').element.nextElementSibling).toBe(wrapper.get('input#name').element);
});
