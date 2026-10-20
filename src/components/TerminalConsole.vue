<script setup lang="ts">

import { onBeforeUnmount, onMounted, useTemplateRef, watch } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

import { useTerminalStore } from '@/store';
import { t } from '@/i18n';

const store = useTerminalStore();
const panel = useTemplateRef<HTMLElement>('panel');
const terminal = new Terminal({
  allowTransparency: true,
  scrollback: 2000,
  fontFamily: 'monospace',
});

const fit = new FitAddon();
terminal.loadAddon(fit);
const encoder = new TextEncoder();
let observer: ResizeObserver | undefined;

function resize() {
  if (panel.value?.clientWidth && panel.value.clientHeight) {
    fit.fit();
    store.resize(terminal.cols, terminal.rows);
  }
}

function appearance() {
  const settings = store.settings;
  if (!settings) {
    return;
  }

  const alpha = Math.round((1 - settings.transparency / 100) * 255)
    .toString(16)
    .padStart(2, '0');

  terminal.options.theme = {
    foreground: settings.textColor,
    background: `${settings.backgroundColor}${alpha}`,
    cursor: settings.highlightColor,
    selectionBackground: `${settings.highlightColor}66`,
  };
  terminal.options.fontSize = settings.fontSize;
  terminal.options.lineHeight = settings.lineHeight;

  resize();
}

watch(() => store.settings, appearance, { deep: true });

watch(
  () => store.cleared,
  () => terminal.clear(),
);

watch(
  () => store.connection,
  (connection) => {
    if (connection === 'open') {
      resize();
      terminal.focus();
    }
  },
);

onMounted(() => {
  terminal.open(panel.value!);
  appearance();
  terminal.onData((data) => store.send(encoder.encode(data)));
  terminal.onBinary((data) =>
    store.send(Uint8Array.from(data, (character) => character.codePointAt(0)!)),
  );

  store.setOutput((data) => terminal.write(data));
  observer = new ResizeObserver(resize);
  observer.observe(panel.value!);
  resize();
  store.connect();
});

onBeforeUnmount(() => {
  observer?.disconnect();
  store.setOutput(null);
  store.disconnect();
  terminal.dispose();
});

defineExpose({
  focus: () => {
    resize();
    terminal.focus();
  },
});

</script>

<template>
  <div
    ref="panel"
    class="terminal_console"
    :aria-label="t('terminalConsole')"
    data-testid="terminal_output"
  ></div>
</template>

<style scoped lang="scss">

.terminal_console {
  @extend %terminal_panel;
  overflow: hidden;
}

</style>
