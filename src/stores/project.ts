import { defineStore } from 'pinia';
import { Project } from '../types/project';

const sanitizeProject = (project: any): Project => {
  return {
    ...project,
    database: project.database || { 
      type: 'none', 
      port: 3307,
      databaseName: project.database?.databaseName || '',
      username: project.database?.username || 'root',
      password: project.database?.password || '',
      initSqlPath: project.database?.initSqlPath || ''
    },
    externalDirs: project.externalDirs || ['vendor', 'storage'],
  };
};

export const useProjectStore = defineStore('project', {
  state: () => {
    const saved = localStorage.getItem('currentProject');
    let currentProject = saved ? JSON.parse(saved) : null;
    if (currentProject) {
      currentProject = sanitizeProject(currentProject);
    }
    return {
      currentProject: currentProject as Project | null,
      recentProjects: [] as string[],
    };
  },
  actions: {
    setCurrentProject(project: Project) {
      this.currentProject = sanitizeProject(project);
      localStorage.setItem('currentProject', JSON.stringify(this.currentProject));
    },
    updateProject(updates: Partial<Project>) {
      if (this.currentProject) {
        this.currentProject = sanitizeProject({ ...this.currentProject, ...updates });
        localStorage.setItem('currentProject', JSON.stringify(this.currentProject));
      }
    },
    async loadProject(path: string) {
      // Logic to load project from disk via Tauri
    },
    async saveProject() {
      // Logic to save project to disk via Tauri
    }
  }
});
