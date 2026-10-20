<script setup lang="ts">

import { NasModal } from '@valeriogc/nas-base-app';
import { t } from '@/i18n';

import CommonIcon from './CommonIcon.vue';
import closeIcon from '@/assets/icons/close.svg';

defineProps<{ title: string }>();
const open = defineModel<boolean>({ required: true });

</script>

<template>
  <Teleport to="body">
    <NasModal v-if="open" :label="title" @close="open = false">
      <section class="common_modal app_surface">
        <header class="common_modal_header">
          <h2>{{ title }}</h2>
          <button
            type="button"
            class="common_modal_close"
            :aria-label="t('close')"
            data-testid="common_modal_close"
            @click="open = false"
          >
            <CommonIcon :src="closeIcon" size="small" />
          </button>
        </header>
        <div class="modal_body"><slot></slot></div>
      </section>
    </NasModal>
  </Teleport>
</template>

<style scoped lang="scss">

.common_modal {
  @extend %panel;
  width: 100%;
  max-width: 40rem;
  padding: $space_6;
  border-radius: 1.25rem;
  box-shadow: 0 18px 50px #00000054;
}

.common_modal_header {
  @extend %fx_between;
  gap: $space_4;
  margin-bottom: $space_4;
  h2 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 650;
  }
}

.common_modal_close {
  @extend %icon_button;
}

.modal_body {
  color: var(--surface-muted);
  font-size: $text_label;
  line-height: 1.5;
}

</style>
