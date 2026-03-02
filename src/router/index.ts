/**
 * ---
 * role: Vue Router 配置，3 个页面的线性流程路由
 * depends:
 *   - vue-router
 *   - ../views/WelcomeView.vue
 *   - ../views/ProgressView.vue
 *   - ../views/CompleteView.vue
 * exports:
 *   - router (default)
 * status: IMPLEMENTED
 * functions:
 *   - 路由定义：
 *     / → WelcomeView（欢迎页，可后退）
 *     /progress → ProgressView（安装进行中，不可后退）
 *     /complete → CompleteView（完成页）
 *   - 导航守卫：
 *     /progress 不允许浏览器后退
 * ---
 */

import { createRouter, createWebHistory } from 'vue-router'
import WelcomeView from '../views/WelcomeView.vue'
import ProgressView from '../views/ProgressView.vue'
import CompleteView from '../views/CompleteView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'Welcome', component: WelcomeView },
    { path: '/progress', name: 'Progress', component: ProgressView },
    { path: '/complete', name: 'Complete', component: CompleteView },
  ],
})

export default router
