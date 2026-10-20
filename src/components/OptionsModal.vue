<script setup lang="ts">

import { ref } from 'vue';
import { t } from '@/i18n';
import { useTerminalStore } from '@/store';
import type { TerminalSettings } from '@/types';

import CommonField from '@/components/common/CommonField.vue';
import CommonModal from '@/components/common/CommonModal.vue';
import CommonSaveFooter from '@/components/common/CommonSaveFooter.vue';
import TerminalStepper from '@/components/TerminalStepper.vue';

type ColourKey = 'textColor' | 'backgroundColor' | 'highlightColor';

const COLOURS: { key: ColourKey; id: string; label: string }[] = [
  { key: 'textColor', id: 'terminal_text_color', label: 'terminalTextColor' },
  { key: 'backgroundColor', id: 'terminal_background_color', label: 'terminalBackgroundColor' },
  { key: 'highlightColor', id: 'terminal_highlight_color', label: 'terminalHighlightColor' },
];

const open = defineModel<boolean>({ required: true });

const terminal = useTerminalStore();
const draft = ref<TerminalSettings | null>(terminal.settings ? { ...terminal.settings } : null);

async function save() {
  if (draft.value) {
    await terminal.saveSettings({ ...draft.value });
  }
}

</script>

<template>
  <CommonModal v-model="open" :title="t('terminalSettings')">
    <form
      v-if="draft"
      class="app_surface app_stack"
      data-testid="terminal_settings"
      @submit.prevent="save"
    >
      <CommonField>
        <label for="terminal_transparency">
          {{ t('terminalTransparency') }} ({{ draft.transparency }}%)
        </label>
        <input
          id="terminal_transparency"
          v-model.number="draft.transparency"
          type="range"
          min="0"
          max="100"
          step="5"
          data-testid="terminal_transparency"
        />
      </CommonField>

      <div class="terminal_colours">
        <CommonField v-for="colour in COLOURS" :key="colour.key" class="terminal_colour">
          <label :for="colour.id">{{ t(colour.label) }}</label>
          <input
            :id="colour.id"
            v-model="draft[colour.key]"
            type="color"
            :data-testid="colour.id"
          />
        </CommonField>
      </div>

      <CommonField class="terminal_inline">
        <label for="terminal_font_size">{{ t('terminalFontSize') }}</label>
        <TerminalStepper
          id="terminal_font_size"
          v-model="draft.fontSize"
          :min="10"
          :max="32"
          :step="1"
        />
      </CommonField>

      <CommonField class="terminal_inline">
        <label for="terminal_line_height">{{ t('terminalLineHeight') }}</label>
        <TerminalStepper
          id="terminal_line_height"
          v-model="draft.lineHeight"
          :min="1"
          :max="2.5"
          :step="0.1"
        />
      </CommonField>

      <p v-if="terminal.error" role="alert" class="app_error">{{ terminal.error }}</p>

      <CommonSaveFooter :busy="terminal.saving" testid="terminal_settings_save" />
    </form>
  </CommonModal>
</template>

<style scoped lang="scss">

.terminal_inline {
  @extend %fx_between;
  flex-direction: row;

  .terminal_stepper {
    flex: none;
    width: $metric_min_width + $space_10;
  }
}

.terminal_colours {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: $space_3;

  .terminal_colour {
    @extend %panel_sunken;
    display: grid;
    grid-template-rows: 1fr auto;
    gap: $space_3;
    padding: $space_3;
  }

  label {
    align-items: flex-start;
    font-size: $text_label;
    color: var(--surface-muted);
  }

  input {
    @extend %control;
    width: $space_10;
    height: $space_10;
    padding: $space_2 * 0.5;
    appearance: none;
    cursor: pointer;

    &:hover {
      border-color: var(--accent);
    }

    &::-webkit-color-swatch-wrapper {
      padding: 0;
    }

    &::-webkit-color-swatch {
      border: 0;
      border-radius: $radius_control - $space_2 * 0.5;
    }

    &::-moz-color-swatch {
      border: 0;
      border-radius: $radius_control - $space_2 * 0.5;
    }
  }
}

@media (max-width: $breakpoint_small) {
  .terminal_colours {
    grid-template-columns: 1fr;

    .terminal_colour {
      grid-template-columns: minmax(0, 1fr) $space_10;
      grid-template-rows: auto;
      align-items: center;
    }
  }
}

</style>
