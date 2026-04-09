<template>
  <transition name="slide-fade">
    <div v-if="store.progress !== null" class="progress-wrapper">
      <div class="progress-info">
        <span class="status-badge" :class="{ pulse: store.isCompiling }">
          {{ store.status }}
        </span>
        <span class="percentage">{{ store.progress }}%</span>
      </div>
      <div class="progress-track">
        <div 
          class="progress-fill" 
          :style="{ width: store.progress + '%' }"
        >
          <div class="shimmer"></div>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { useCompilerStore } from '../stores/compiler';

const store = useCompilerStore();
</script>

<style scoped>
.progress-wrapper {
  background: rgba(15, 23, 42, 0.9);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  padding: 12px 24px;
  position: relative;
  z-index: 100;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.status-badge {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--accent);
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-badge::before {
  content: '';
  display: inline-block;
  width: 6px;
  height: 6px;
  background: var(--accent);
  border-radius: 50%;
}

.pulse::before {
  animation: pulse-dot 1.5s infinite;
}

.percentage {
  font-family: 'JetBrains Mono', monospace;
  font-weight: 800;
  font-size: 0.875rem;
  color: var(--text-main);
}

.progress-track {
  height: 6px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 3px;
  overflow: hidden;
  position: relative;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent) 0%, #60a5fa 100%);
  border-radius: 3px;
  transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
}

.shimmer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.2) 50%,
    transparent 100%
  );
  animation: shimmer 2s infinite;
}

@keyframes pulse-dot {
  0% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.5); opacity: 0.5; }
  100% { transform: scale(1); opacity: 1; }
}

@keyframes shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}

.slide-fade-enter-active {
  transition: all 0.3s ease-out;
}

.slide-fade-leave-active {
  transition: all 0.2s cubic-bezier(1, 0.5, 0.8, 1);
}

.slide-fade-enter-from,
.slide-fade-leave-to {
  transform: translateY(-10px);
  opacity: 0;
}
</style>
