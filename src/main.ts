/**
 * ---
 * role: Vue 应用主入口，初始化应用和插件
 * depends:
 *   - ./App.vue
 *   - ./router/index.ts
 *   - pinia
 * exports:
 *   - app
 * status: PENDING
 * functions:
 *   - main(): void
 *     创建 Vue 应用实例，注册 Pinia 和 Router，挂载到 #app
 * ---
 */

import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "./router";
import "./style.css";

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
