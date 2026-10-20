<script setup lang="ts">

const props = withDefaults(
  defineProps<{
    src: string;
    label?: string;
    size?: 'small' | 'medium';
  }>(),
  { size: 'medium', label: undefined },
);

</script>

<template>
  <span
    class="common_icon"
    :class="size === 'small' ? 'common_icon_small' : 'common_icon_medium'"
    :style="{ backgroundImage: 'url(' + JSON.stringify(props.src) + ')' }"
    :role="label ? 'img' : undefined"
    :aria-label="label"
    :aria-hidden="label ? undefined : 'true'"
  ></span>
</template>

<style scoped lang="scss">

.common_icon {
  display: inline-block;
  flex-shrink: 0;
  background-position: center;
  background-size: contain;
  background-repeat: no-repeat;
  &.common_icon_small {
    width: 1rem;
    height: 1rem;
  }

  &.common_icon_medium {
    width: 1.25rem;
    height: 1.25rem;
  }
}

</style>
