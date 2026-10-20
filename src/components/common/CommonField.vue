<template>
  <div class="common_field">
    <slot></slot>
  </div>
</template>

<style scoped lang="scss">

.common_field {
  @extend %stack;
  align-items: stretch;
  gap: $space_2;
}

</style>
