<script setup lang="ts" generic="T extends string">

import { t } from '@/i18n';

interface Option {
  value: T;
  label: string;
}

interface Props {
  options: Option[];
  label: string;
  testid?: string;
}

withDefaults(defineProps<Props>(), { testid: undefined });

const model = defineModel<T>({ required: true });

</script>

<template>
  <fieldset class="common_segmented" :aria-label="label">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      class="common_segmented_option"
      :aria-pressed="model === option.value"
      :data-testid="testid && `${testid}_${option.value}`"
      @click="model = option.value"
    >
      {{ t(option.label) }}
    </button>
  </fieldset>
</template>

<style scoped lang="scss">

.common_segmented {
  @extend %fieldset_reset;
  @extend %panel_sunken;
  display: flex;
  flex-wrap: wrap;
  gap: $space_2;
  padding: $space_2 * 0.5;

  &_option {
    @extend %button;
    flex: 1;
    border-color: transparent;
    background: transparent;
    white-space: nowrap;

    &[aria-pressed='true'] {
      border-color: var(--accent);
      background: var(--surface);
      color: var(--accent);
    }
  }
}

</style>
