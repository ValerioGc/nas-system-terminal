<script setup lang="ts">

import { nextTick, useTemplateRef, watch } from 'vue';
import type { TerminalEntry } from '@/types';

interface Props {
  lines: TerminalEntry[];
  received: number;
  waiting: string;
  label: string;
  testid: string;
}

const FOLLOW_MARGIN = 8;

const props = defineProps<Props>();
const panel = useTemplateRef<HTMLElement>('panel');

watch(
  () => props.received,
  async () => {
    const element = panel.value;
    if (!element) {
      return;
    }
    const atEnd = element.scrollHeight - element.scrollTop - element.clientHeight <= FOLLOW_MARGIN;
    await nextTick();
    if (atEnd) {
      element.scrollTop = element.scrollHeight;
    }
  },
);

</script>

<template>
  <div
    ref="panel"
    class="terminal_output"
    role="log"
    :aria-label="props.label"
    :data-testid="props.testid"
  >
    <p v-if="!props.lines.length" class="terminal_waiting">{{ props.waiting }}</p>
    <p
      v-for="(entry, index) in props.lines"
      :key="index"
      class="terminal_line"
      v-text="entry.text"
    ></p>
  </div>
</template>

<style scoped lang="scss">

.terminal_output {
  @extend %terminal_panel;
  overflow: auto;
  color: var(--terminal-text, var(--theme-foreground));
  font-family: $font_mono;
  font-size: var(--terminal-font-size, #{$text_label});
  line-height: var(--terminal-line-height, 1.5);

  *::selection {
    background: var(--terminal-highlight, var(--accent));
  }
}

.terminal_line {
  min-height: 1lh;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.terminal_waiting {
  @extend %muted;
  font-size: inherit;
}

</style>
