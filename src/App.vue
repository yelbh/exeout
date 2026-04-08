<template>
  <div class="app-container">
    <aside class="sidebar">
      <div class="logo-container">
        <div class="logo-icon">EXE</div>
        <span>Output Studio</span>
      </div>
      <nav>
        <router-link to="/" class="nav-item" :class="{ active: route.name === 'dashboard' }">
          Tableau de bord
        </router-link>
        <router-link to="/config" class="nav-item" :class="{ active: route.name === 'config' }">
          Configuration
        </router-link>
        <router-link to="/settings" class="nav-item" :class="{ active: route.name === 'settings' }">
          Paramètres
        </router-link>
      </nav>
    </aside>

    <main class="content">
      <header class="app-header">
        <h1>{{ projectStore.currentProject?.name || 'Aucun Projet' }}</h1>
        <div class="header-actions">
          <button @click="openProject">Ouvrir</button>
          <button class="primary" @click="compileProject">Compiler</button>
          <button v-if="lastOutputPath" class="success" @click="deployProject">🚀 Déployer</button>
          <button @click="previewProject">Aperçu</button>
        </div>
      </header>

      <!-- Progress Bar -->
      <div v-if="compilationProgress !== null" class="progress-container">
        <div class="progress-bar" :style="{ width: compilationProgress + '%' }"></div>
        <span class="progress-text">{{ compilationProgress }}%</span>
      </div>

      <section class="main-view">
        <router-view />
        
        <div v-if="!projectStore.currentProject && route.name === 'config'" class="no-project">
          <h2>Prêt à commencer ?</h2>
          <p>Choisissez un projet sur le tableau de bord ou ouvrez-en un pour accéder à la configuration.</p>
          <button @click="router.push('/')" style="margin-top: 1rem">Aller au dashboard</button>
        </div>
      </section>

      <ConsoleLog v-show="compilerStore.logs.length > 0" />
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useProjectStore } from './stores/project';
import { useCompilerStore } from './stores/compiler';
import ConsoleLog from './components/ConsoleLog.vue';
import { invoke } from '@tauri-apps/api/tauri';
import { dialog, shell } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { useRouter, useRoute } from 'vue-router';
import { checkUpdate, installUpdate } from '@tauri-apps/api/updater';
import { relaunch } from '@tauri-apps/api/process';

const router = useRouter();
const route = useRoute();
const projectStore = useProjectStore();
const compilerStore = useCompilerStore();

const compilationProgress = ref<number | null>(null);
const lastOutputPath = ref<string | null>(null);
const lastJsonPath = ref<string | null>(null);

onMounted(async () => {
  // Vérification automatique des mises à jour au démarrage
  try {
    const { shouldUpdate, manifest } = await checkUpdate();
    if (shouldUpdate) {
      const yes = await dialog.confirm(
        `Une nouvelle version (${manifest?.version}) est disponible !\n\n${manifest?.body}\n\nVoulez-vous mettre à jour maintenant ?`,
        { title: 'Mise à jour disponible', type: 'info' }
      );
      if (yes) {
        compilerStore.addLog('info', `Téléchargement de la version ${manifest?.version}...`);
        await installUpdate();
        await relaunch();
      }
    }
  } catch (e) {
    // Silencieux en cas d'échec réseau (mode hors-ligne)
    console.warn('Vérification de mise à jour impossible :', e);
  }

  await listen<number>('compilation-progress', (event) => {
    compilationProgress.value = event.payload;
    if (event.payload === 100) {
      setTimeout(() => {
        compilationProgress.value = null;
      }, 3000);
    }
  });
  await listen<string>('compilation-log', (event) => {
    compilerStore.addLog('info', event.payload);
  });
});

const newProject = () => {
  projectStore.currentProject = null;
  router.push('/');
};

const openProject = async () => {
  const selected = await dialog.open({
    directory: false,
    multiple: false,
    title: 'Ouvrir un projet',
    filters: [{ name: 'Projets ExeOutput', extensions: ['exeoutput', 'json'] }]
  });
  if (selected) {
    try {
      const config: any = await invoke('load_config', { path: selected as string });
      
      const sourceDir = config.sourceDir || '';
      const name = config.exeName || (selected as string).split(/[\\/]/).pop()?.replace('.exeoutput', '') || 'Projet';
      
      projectStore.setCurrentProject({
        name,
        sourceDir,
        version: config.phpVersion || '1.0.0',
        entryPoint: config.entryPoint || 'index.php',
        publicDir: config.publicDir || '',
        externalDirs: config.externalDirs || ['vendor', 'storage'],
        iconPath: config.iconPath || '',
        database: config.database || { type: 'none', port: 3307 },
        updateUrl: config.updateUrl || '',
        envVars: config.envVars || { "DB_HOST": "127.0.0.1", "STATION_NAME": "STATION-01" },
      });
      
      // Pass the loaded config to CompilerConfig component
      localStorage.setItem('loadedConfig', JSON.stringify(config));
      
      router.push('/config');
      compilerStore.addLog('info', `Projet ouvert : ${selected}`);
    } catch (e) {
      alert(`Erreur lors de l'ouverture : ${e}`);
    }
  }
};

const previewProject = async () => {
  if (!projectStore.currentProject) {
    alert('Veuillez d\'abord ouvrir ou créer un projet.');
    return;
  }
  compilerStore.addLog('info', 'Lancement de l\'aperçu...');
  try {
    const port = await invoke('preview_project', {
      source: projectStore.currentProject.sourceDir,
      databaseConfig: projectStore.currentProject.database || null,
    });
    compilerStore.addLog('info', `Serveur d'aperçu lancé sur le port ${port}`);
    await shell.open(`http://127.0.0.1:${port}`);
  } catch (e) {
    compilerStore.addLog('error', e as string);
  }
};

const compileProject = async () => {
  if (!projectStore.currentProject) {
    alert('Veuillez d\'abord ouvrir ou créer un projet.');
    return;
  }

  const outFile = await dialog.save({
    title: 'Enregistrer l\'exécutable sous...',
    defaultPath: `${projectStore.currentProject.name}.exe`,
    filters: [{
      name: 'Exécutable Windows',
      extensions: ['exe']
    }]
  });

  if (!outFile) return;

  lastOutputPath.value = null;
  lastJsonPath.value = null;
  compilationProgress.value = 0; // Force immediate visibility
  compilerStore.addLog('info', 'Début de la compilation...');
  try {
    const result = await invoke('compile_project', {
      name: projectStore.currentProject.name,
      version: projectStore.currentProject.version || '1.0.0',
      source: projectStore.currentProject.sourceDir,
      output: outFile as string,
      entryPoint: projectStore.currentProject.entryPoint,
      publicDir: projectStore.currentProject.publicDir,
      externalDirs: projectStore.currentProject.externalDirs || [],
      iconPath: projectStore.currentProject.iconPath || null,
      databaseConfig: projectStore.currentProject.database || null,
      updateUrl: projectStore.currentProject.updateUrl || null,
      notes: projectStore.currentProject.notes || null,
      envVars: projectStore.currentProject.envVars || {},
    });
    
    lastOutputPath.value = outFile as string;
    // Derive JSON path from updateUrl or exe name
    const url = projectStore.currentProject.updateUrl;
    const jsonName = url ? url.split('/').pop() : `${projectStore.currentProject.name}.json`;
    lastJsonPath.value = (outFile as string).replace(/[\\/][^\\/]+$/, `/${jsonName}`);
    
    compilerStore.addLog('info', result as string);
    alert(result);
  } catch (e) {
    compilationProgress.value = null; // Hide bar on error
    compilerStore.addLog('error', `Erreur de compilation : ${e}`);
    alert(`Erreur : ${e}`);
  }
};

const deployProject = async () => {
  if (!projectStore.currentProject?.server || !lastOutputPath.value || !lastJsonPath.value) return;

  const confirm = await dialog.confirm(
    `Voulez-vous envoyer les fichiers sur ${projectStore.currentProject.server.host} ?`,
    { title: 'Confirmation de déploiement', type: 'info' }
  );

  if (!confirm) return;

  compilerStore.addLog('info', 'Début du déploiement SFTP...');
  try {
    const result = await invoke('deploy_project', {
      exePath: lastOutputPath.value,
      jsonPath: lastJsonPath.value,
      server: projectStore.currentProject.server
    });
    compilerStore.addLog('info', result as string);
    alert(result);
  } catch (e) {
    compilerStore.addLog('error', `Erreur de déploiement : ${e}`);
    alert(`Erreur : ${e}`);
  }
};
</script>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.logo-icon {
  background: var(--accent);
  color: white;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 0.8rem;
  font-weight: 900;
}

.progress-container {
  height: 24px;
  background: var(--bg-sidebar);
  border-bottom: 1px solid var(--border);
  position: relative;
  overflow: hidden;
}

.progress-bar {
  height: 100%;
  background: var(--success);
  transition: width 0.3s ease-out;
}

.progress-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 0.75rem;
  font-weight: 700;
  color: white;
  text-shadow: 0 1px 2px rgba(0,0,0,0.5);
}
</style>
