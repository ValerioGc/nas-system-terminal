import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { guardFrame } from '@valeriogc/nas-base-app';
import App from '@/App.vue';
import '@/styles/main.scss';

guardFrame();
createApp(App).use(createPinia()).mount('#app');
