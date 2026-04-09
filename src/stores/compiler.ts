import { defineStore } from 'pinia';
import { LogEntry } from '../types/project';

export const useCompilerStore = defineStore('compiler', {
  state: () => ({
    isCompiling: false,
    progress: null as number | null,
    logs: [] as LogEntry[],
    status: 'Idle',
  }),
  actions: {
    addLog(level: LogEntry['level'], message: string) {
      this.logs.push({ level, message, timestamp: Date.now() });
    },
    clearLogs() {
      this.logs = [];
    },
    setProgress(val: number | null) {
      this.progress = val;
      if (val === 100) {
        this.isCompiling = false;
        this.status = 'Terminé';
        setTimeout(() => {
          this.progress = null;
        }, 3000);
      } else if (val !== null) {
        this.isCompiling = true;
        this.status = 'Compilation...';
      } else {
        this.isCompiling = false;
      }
    },
    resetProgress() {
      this.progress = null;
      this.isCompiling = false;
      this.status = 'Idle';
    }
  }
});
