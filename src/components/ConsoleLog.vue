<template>
  <div class="console-log">
    <div class="log-header">
      <h3>Console de Compilation</h3>
      <button @click="store.clearLogs">Effacer</button>
    </div>
    <div class="log-container" ref="logBox">
      <div v-for="(log, i) in store.logs" :key="i" :class="['log-item', log.level]">
        <span class="timestamp">[{{ new Date(log.timestamp).toLocaleTimeString() }}]</span>
        <span class="message">{{ log.message }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onUpdated } from 'vue';
import { useCompilerStore } from '../stores/compiler';

const store = useCompilerStore();
const logBox = ref<HTMLElement | null>(null);

onUpdated(() => {
  if (logBox.value) {
    logBox.value.scrollTop = logBox.value.scrollHeight;
  }
});
</script>

<style scoped>
.console-container {
  height: 200px;
  background: #020617;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
}

.console-header {
  padding: 0.5rem 1rem;
  background: var(--bg-sidebar);
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-muted);
}

.log-list {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.875rem;
}

.log-item {
  margin-bottom: 0.25rem;
  display: flex;
  gap: 1rem;
}

.timestamp { color: var(--text-muted); opacity: 0.5; }
.info { color: var(--accent); }
.success { color: var(--success); }
.warning { color: var(--warning); }
.error { color: var(--error); }
</style>
