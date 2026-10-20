<script setup lang="ts">

import { t } from '@/i18n';

interface Props {
  busy: boolean;
  testid?: string;
}

withDefaults(defineProps<Props>(), { testid: undefined });

</script>

<template>
  <div class="common_save_footer">
    <button type="submit" class="primary" :disabled="busy" :data-testid="testid">
      {{ t('save') }}
    </button>
  </div>
</template>

<style scoped lang="scss">

.common_save_footer {
  @extend %fx_row_center;
  flex-wrap: wrap;
  gap: $space_2;
  padding-block: $space_2;

  button {
    margin-left: auto;
  }
}

</style>
