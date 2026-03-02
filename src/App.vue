<!--
---
role: Vue 根组件，简单的路由容器
depends:
  - ./router/index.ts
exports:
  - default
status: IMPLEMENTED
functions:
  - setup(): void
    仅渲染 <router-view />，无全局 layout 复杂度
---
-->

<script setup lang="ts">
// 简单根组件，无需额外逻辑
</script>

<template>
  <router-view />
</template>
