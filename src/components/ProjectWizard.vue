<template>
  <div class="wizard-container">
    <h2>Nouveau Projet</h2>
    <div class="form-group">
      <label>Nom du Projet</label>
      <input v-model="project.name" type="text" placeholder="Mon App PHP" />
    </div>
    <div class="form-group">
      <label>Dossier Source</label>
      <div class="input-with-button">
        <input v-model="project.sourceDir" type="text" readonly />
        <button @click="selectDir">Parcourir</button>
      </div>
    </div>
    <div class="form-group">
      <label>Icône</label>
      <div class="input-with-button">
        <input v-model="project.icon" type="text" readonly />
        <button @click="selectIcon">Sélectionner</button>
      </div>
    </div>
    <div class="form-group">
      <label>Version</label>
      <input v-model="project.version" type="text" placeholder="1.0.0" />
    </div>
    <div class="form-group">
      <label>Point d'entrée</label>
      <div class="input-with-button">
        <input v-model="project.entryPoint" type="text" placeholder="index.php" />
        <button @click="selectEntryPoint" :disabled="!project.sourceDir">Parcourir</button>
      </div>
      <small v-if="!project.sourceDir" class="hint">Sélectionnez d'abord un dossier source</small>
    </div>
    <div class="form-group">
      <label>Dossier Public (Document Root)</label>
      <input v-model="project.publicDir" type="text" placeholder="public (optionnel)" />
      <small class="hint">Sous-dossier servant de racine web (laissez vide pour la racine)</small>
    </div>

    <!-- Database Configuration -->
    <div class="database-section">
      <div class="section-header">
        <h3>Base de données</h3>
        <select v-model="project.database!.type" class="db-type-select">
          <option value="none">Aucune</option>
          <option value="mariadb">MariaDB (Portable)</option>
          <option value="sqlite">SQLite</option>
        </select>
      </div>

      <div v-if="project.database?.type !== 'none'" class="db-fields">
        <div v-if="project.database?.type === 'mariadb'" class="db-grid">
          <div class="form-group">
            <label>Port</label>
            <input v-model.number="project.database!.port" type="number" placeholder="3307" />
          </div>
          <div class="form-group">
            <label>Nom de la base</label>
            <input v-model="project.database!.databaseName" type="text" placeholder="ma_base" />
          </div>
          <div class="form-group">
            <label>Utilisateur</label>
            <input v-model="project.database!.username" type="text" placeholder="root" />
          </div>
          <div class="form-group">
            <label>Mot de passe</label>
            <input v-model="project.database!.password" type="password" />
          </div>
        </div>

        <div class="form-group">
          <label>Script SQL initial</label>
          <div class="input-with-button">
            <input v-model="project.database!.initSqlPath" type="text" readonly placeholder="db.sql (optionnel)" />
            <button @click="selectSqlFile" :disabled="!project.sourceDir">Parcourir</button>
          </div>
          <small class="hint">Exécuté au premier lancement de l'application</small>
        </div>
      </div>
    </div>

    <div v-if="error" class="error">{{ error }}</div>
    <button class="create-btn primary" :disabled="!isValid" @click="create">Créer le projet</button>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { useProjectStore } from '../stores/project';
import { Project } from '../types/project';
import { dialog } from '@tauri-apps/api';
import { useRouter } from 'vue-router';

const router = useRouter();
const store = useProjectStore();
const error = ref('');
const project = reactive<Project>(store.currentProject ? { ...store.currentProject } : {
  name: '',
  sourceDir: '',
  version: '1.0.0',
  icon: '',
  entryPoint: 'index.php',
  publicDir: '',
  externalDirs: ['vendor', 'storage'],
});

// Sync partial updates to store to avoid losing data while typing
import { watch } from 'vue';
watch(project, (newVal) => {
  if (store.currentProject) {
    store.updateProject(newVal);
  }
}, { deep: true });

const isValid = computed(() => {
  return project.name.length > 0 && project.sourceDir.length > 0;
});

const selectIcon = async () => {
  const selected = await dialog.open({
    filters: [{ name: 'Images', extensions: ['png', 'ico'] }],
  });
  if (selected) project.icon = selected as string;
};

const selectDir = async () => {
  const selected = await dialog.open({
    directory: true,
    multiple: false,
  });
  if (selected) {
    project.sourceDir = selected as string;
  }
};

const selectEntryPoint = async () => {
  if (!project.sourceDir) return;
  
  const selected = await dialog.open({
    defaultPath: project.sourceDir,
    filters: [{ name: 'Fichiers Web', extensions: ['php', 'html', 'htm'] }],
    multiple: false,
  });
  
  if (selected) {
    const fullPath = selected as string;
    // Make it relative to sourceDir
    if (fullPath.startsWith(project.sourceDir)) {
      let rel = fullPath.substring(project.sourceDir.length);
      if (rel.startsWith('/') || rel.startsWith('\\')) rel = rel.substring(1);
      project.entryPoint = rel;
    } else {
      project.entryPoint = fullPath.split(/[\\/]/).pop() || 'index.php';
    }
  }
};

const selectSqlFile = async () => {
  if (!project.sourceDir) return;
  const selected = await dialog.open({
    defaultPath: project.sourceDir,
    filters: [{ name: 'SQL', extensions: ['sql'] }],
    multiple: false,
  });
  if (selected) project.database!.initSqlPath = selected as string;
};

const create = () => {
  if (project.name && project.sourceDir) {
    store.setCurrentProject({ ...project });
    router.push('/config');
  }
};
</script>

<style scoped>
.wizard-container {
  max-width: 600px;
  margin: 0 auto;
  background: var(--bg-card);
  padding: 2.5rem;
  border-radius: 1rem;
  border: 1px solid var(--border);
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
}

h2 {
  margin-top: 0;
  margin-bottom: 2rem;
  font-size: 1.5rem;
  font-weight: 700;
}

.form-group {
  margin-bottom: 1.5rem;
}

label {
  display: block;
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-muted);
}

input {
  width: 100%;
  padding: 0.75rem 1rem;
  background: var(--bg-dark);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text-main);
  box-sizing: border-box;
}

input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

.input-with-button {
  display: flex;
  gap: 0.5rem;
}

.database-section {
  background: rgba(15, 23, 42, 0.3);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}

.section-header h3 {
  margin: 0;
  font-size: 1.1rem;
}

.db-type-select {
  padding: 0.5rem;
  background: var(--bg-dark);
  border: 1px solid var(--border);
  border-radius: 0.375rem;
  color: var(--text-main);
  width: 200px;
}

.db-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}

.hint {
  display: block;
  margin-top: 0.25rem;
  color: var(--text-muted);
  font-size: 0.75rem;
}

.create-btn {
  width: 100%;
  margin-top: 1rem;
  padding: 1rem;
  background: var(--accent);
  color: white;
  border: none;
  font-weight: 600;
}

.create-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: var(--border);
}
</style>
