import { createRouter, createWebHashHistory } from 'vue-router';

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: () => import('../components/ProjectWizard.vue'),
    },
    {
      path: '/config',
      name: 'config',
      component: () => import('../components/CompilerConfig.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('../components/Settings.vue'),
    }
  ],
});

export default router;
