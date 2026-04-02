import { defineStore } from 'pinia';
import { LogEntry } from '../types/project';

export const useCompilerStore = defineStore('compiler', {
  state: () => ({
    isCompiling: false,
    progress: 0,
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
    async startCompile() {
      this.isCompiling = true;
      this.progress = 0;
      this.status = 'Compiling...';
    }
  }
});
