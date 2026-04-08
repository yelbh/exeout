<template>
  <div class="compiler-config">
    <h2>Configuration de Compilation</h2>
    <div class="tabs">
      <button :class="{ active: tab === 'general' }" @click="tab = 'general'">Général</button>
      <button :class="{ active: tab === 'php' }" @click="tab = 'php'">PHP</button>
      <button :class="{ active: tab === 'server' }" @click="tab = 'server'">Serveur</button>
      <button :class="{ active: tab === 'security' }" @click="tab = 'security'">Sécurité</button>
      <button :class="{ active: tab === 'files' }" @click="tab = 'files'">Fichiers</button>
      <button :class="{ active: tab === 'database' }" @click="tab = 'database'">Base de données</button>
      <button :class="{ active: tab === 'updates' }" @click="tab = 'updates'">Mises à jour</button>
      <button :class="{ active: tab === 'server_config' }" @click="tab = 'server_config'">Serveur (SFTP)</button>
      <button :class="{ active: tab === 'environment' }" @click="tab = 'environment'">Environnement (.env)</button>
      <button :class="{ active: tab === 'advanced' }" @click="tab = 'advanced'">Avancé</button>
    </div>
    <div v-if="tab === 'general'" class="tab-content">
      <div class="form-group">
        <label>Nom de l'exécutable</label>
        <input v-model="settings.exeName" type="text" />
      </div>
      <div class="form-group">
        <label>Dossier Source</label>
        <div class="input-with-button">
          <input :value="store.currentProject?.sourceDir" type="text" readonly />
          <button @click="selectDir">Changer</button>
        </div>
      </div>
      <div class="form-group">
        <label>Point d'entrée</label>
        <div class="input-with-button">
          <input v-model="store.currentProject!.entryPoint" type="text" @change="store.updateProject({ entryPoint: store.currentProject!.entryPoint })" />
          <button @click="selectEntryPoint">Parcourir</button>
        </div>
      </div>
      <div class="form-group">
        <label>Dossier Public (Document Root)</label>
        <input v-model="store.currentProject!.publicDir" type="text" @change="store.updateProject({ publicDir: store.currentProject!.publicDir })" />
        <small class="hint">Sous-dossier servant de racine web (ex: public)</small>
      </div>
      <div class="form-group">
        <label>Icône de l'exécutable (.ico)</label>
        <div class="input-with-button">
          <input :value="store.currentProject?.iconPath" type="text" placeholder="Par défaut (Logo ExeOutput)" readonly />
          <button @click="selectIcon">Parcourir</button>
        </div>
      </div>
    </div>
    <div v-if="tab === 'files'" class="tab-content">
      <div class="files-selector">
        <h3>Gestion des dossiers</h3>
        <p class="description">Choisissez les dossiers à sortir de l'exécutable pour optimiser la taille ou la rapidité.</p>
        
        <div v-if="projectDirs.length === 0" class="loading-dirs">Chargement des dossiers du projet...</div>
        
        <div v-else class="dirs-list">
          <div v-for="dir in projectDirs" :key="dir" class="dir-item">
            <label class="checkbox-container">
              <input type="checkbox" :checked="store.currentProject?.externalDirs.includes(dir)" @change="toggleExternal(dir)" />
              <span class="checkmark"></span>
              <span class="dir-name">{{ dir }}</span>
            </label>
            <span class="dir-status" :class="{ external: store.currentProject?.externalDirs.includes(dir) }">
              {{ store.currentProject?.externalDirs.includes(dir) ? 'Externe (data/)' : 'Interne (EXE)' }}
            </span>
          </div>
        </div>
      </div>
    </div>
    <div v-if="tab === 'php'" class="tab-content">
      <div class="form-group">
        <label>Version PHP</label>
        <select v-model="settings.phpVersion">
          <option>8.1</option>
          <option>8.2</option>
          <option>8.3</option>
        </select>
      </div>
    </div>
    <div v-if="tab === 'server'" class="tab-content">
      <div class="form-group">
        <label>Port Local</label>
        <input v-model.number="settings.port" type="number" />
      </div>
    </div>
    <div v-if="tab === 'security'" class="tab-content">
      <div class="form-group">
        <label>Chiffrement AES-256</label>
        <input v-model="settings.encryption" type="checkbox" />
      </div>
    </div>
    <div v-if="tab === 'database' && store.currentProject?.database" class="tab-content">
      <div class="db-config">
        <h3>Configuration de la Base de Données</h3>
        <p class="description">Intégrez une base de données directement dans votre exécutable.</p>
        
        <div class="form-group">
          <label>Moteur de données</label>
          <select v-model="store.currentProject!.database!.type" @change="store.updateProject({ database: store.currentProject!.database })">
            <option value="none">Aucun (Pas de BDD)</option>
            <option value="sqlite">SQLite (Léger, un seul fichier)</option>
            <option value="mariadb">MariaDB Portable (Serveur autonome intégré)</option>
          </select>
        </div>

        <div v-if="store.currentProject.database.type === 'mariadb'" class="mariadb-settings">
          <div class="form-row">
            <div class="form-group flex-1">
              <label>Port du serveur MySQL</label>
              <input v-model.number="store.currentProject.database.port" type="number" @change="store.updateProject({ database: store.currentProject.database })" />
            </div>
            <div class="form-group flex-2">
              <label>Nom de la base (DB_DATABASE)</label>
              <input v-model="store.currentProject.database.databaseName" type="text" placeholder="ma_base_de_donnees" @change="store.updateProject({ database: store.currentProject.database })" />
            </div>
          </div>

          <div class="form-row">
            <div class="form-group flex-1">
              <label>Utilisateur (DB_USERNAME)</label>
              <input v-model="store.currentProject.database.username" type="text" placeholder="root" @change="store.updateProject({ database: store.currentProject.database })" />
            </div>
            <div class="form-group flex-1">
              <label>Mot de passe (DB_PASSWORD)</label>
              <input v-model="store.currentProject.database.password" type="password" placeholder="Laissez vide pour aucun" @change="store.updateProject({ database: store.currentProject.database })" />
            </div>
          </div>
          
          <div class="form-group">
            <label>Fichier SQL d'initialisation (Optionnel)</label>
            <div class="input-with-button">
              <input :value="store.currentProject.database.initSqlPath" type="text" placeholder="Selectionnez un fichier .sql pour importer des données au 1er lancement" readonly />
              <button @click="selectInitSql">Parcourir</button>
            </div>
          </div>
          
          <div class="info-box warning">
            <p><strong>Note :</strong> L'exécutable cherchera <code>mysqld.exe</code> dans le dossier <code>data/mysql</code> lors du lancement.</p>
          </div>
        </div>

        <div v-if="store.currentProject.database.type === 'sqlite'" class="sqlite-settings">
          <div class="info-box info">
            <p>Utilisez <code>database_path('database.sqlite')</code> ou un chemin relatif dans votre config Laravel/Symfony.</p>
          </div>
        </div>
      </div>
    </div>
    <div v-if="tab === 'updates'" class="tab-content">
      <div class="info-box info">
        <p>Afin que vos exécutables finaux puissent se mettre à jour tout seuls chez vos clients, vous devez fournir une URL pointant vers un fichier JSON (ex: version.json).</p>
      </div>
      <div class="form-group">
        <label>URL du fichier JSON de mise à jour (Optionnel)</label>
        <input v-model="store.currentProject!.updateUrl" type="url" placeholder="https://ben-sms.com/maj.json" @change="store.updateProject({ updateUrl: store.currentProject!.updateUrl })" />
        <small class="hint">Laissez vide pour désactiver la mise à jour automatique.</small>
      </div>
      <div class="form-group">
        <label>Notes de cette version (Affichées à l'utilisateur)</label>
        <textarea v-model="store.currentProject!.notes" placeholder="Ex: Correction de bugs mineurs, Amélioration de l'interface..." @change="store.updateProject({ notes: store.currentProject!.notes })"></textarea>
      </div>
    </div>
    <div v-if="tab === 'server_config' && store.currentProject" class="tab-content">
      <h3>Configuration du déploiement SFTP</h3>
      <p class="description">Configurez les accès pour envoyer automatiquement vos fichiers sur votre serveur.</p>
      
      <div class="form-row">
        <div class="form-group flex-2">
          <label>Hôte SSH/SFTP</label>
          <input v-model="store.currentProject.server!.host" type="text" placeholder="ex: node38-ca.n0c.com" />
        </div>
        <div class="form-group flex-1">
          <label>Port</label>
          <input v-model.number="store.currentProject.server!.port" type="number" />
        </div>
      </div>

      <div class="form-row">
        <div class="form-group flex-1">
          <label>Utilisateur</label>
          <input v-model="store.currentProject.server!.user" type="text" />
        </div>
        <div class="form-group flex-1">
          <label>Mot de passe</label>
          <input v-model="store.currentProject.server!.pass" type="password" />
        </div>
      </div>

      <div class="form-group">
        <label>Chemin distant (Dossier cible)</label>
        <input v-model="store.currentProject.server!.remotePath" type="text" placeholder="ex: /home/user/public_html/" />
        <small class="hint">Le dossier où seront déposés le .exe et le .json</small>
      </div>
    </div>
    <div v-if="tab === 'environment' && store.currentProject" class="tab-content">
      <h3>Variables d'Environnement Externes</h3>
      <p class="description">Définissez ici les variables qui seront mises dans le fichier <code>.env.template</code> pour chaque station (ex: DB_HOST, STATION_NAME).</p>
      
      <div class="env-vars-editor">
        <div v-for="(val, key) in store.currentProject.envVars" :key="key" class="env-var-row">
          <input :value="key" type="text" placeholder="CLE" @change="updateEnvKey(key, $event)" />
          <span>=</span>
          <input :value="val" type="text" placeholder="Valeur par défaut" @change="updateEnvValue(key, $event)" />
          <button class="remove-btn" @click="removeEnvVar(key)">×</button>
        </div>
        <button class="add-btn" @click="addEnvVar">+ Ajouter une variable</button>
      </div>

      <div class="info-box info" style="margin-top: 2rem;">
        <p><strong>Note :</strong> Ces variables seront éditables directement sur chaque poste via l'interface de configuration de l'application (Ctrl+Shift+S).</p>
      </div>
    </div>
    <div v-if="tab === 'advanced'" class="tab-content">
      <div class="form-group">
        <label>Niveau de compression (0-9)</label>
        <input v-model.number="settings.compressionLevel" type="range" min="0" max="9" />
      </div>
    </div>
    <button class="save-btn" @click="save">Enregistrer la configuration</button>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useProjectStore } from '../stores/project';
import { useCompilerStore } from '../stores/compiler';
import { invoke } from '@tauri-apps/api/tauri';
import { CompilerSettings } from '../types/project';

const projectDirs = ref<string[]>([]);
const store = useProjectStore();
const compilerStore = useCompilerStore();

interface ExtendedSettings extends CompilerSettings {
  exeName: string;
}

const tab = ref<'general' | 'php' | 'server' | 'security' | 'files' | 'database' | 'updates' | 'server_config' | 'environment' | 'advanced'>('general');
const settings = reactive<ExtendedSettings>({
  exeName: 'MonApp',
  phpVersion: '8.2',
  extensions: [],
  port: 8080,
  timeout: 30,
  encryption: false,
  compressionLevel: 3,
  externalDirs: ['vendor', 'storage'],
});

import { watch } from 'vue';

// Suggestion automatique de l'URL de mise à jour
watch(() => settings.exeName, (newName) => {
  if (store.currentProject && (!store.currentProject.updateUrl || store.currentProject.updateUrl.includes('ben-sms.com'))) {
    const filename = newName.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
    if (filename) {
      store.updateProject({ updateUrl: `https://ben-sms.com/${filename}.json` });
    }
  }
});

const fetchProjectDirs = async () => {
  if (!store.currentProject) return;
  try {
    const dirs: string[] = await invoke('get_project_dirs', { path: store.currentProject.sourceDir });
    projectDirs.value = dirs;
  } catch (e) {
    console.error(e);
  }
};

const toggleExternal = (dir: string) => {
  if (!store.currentProject) return;
  const current = [...store.currentProject.externalDirs];
  const index = current.indexOf(dir);
  if (index > -1) {
    current.splice(index, 1);
  } else {
    current.push(dir);
  }
  store.updateProject({ externalDirs: current });
};

const addEnvVar = () => {
  if (!store.currentProject) return;
  const current = { ...store.currentProject.envVars };
  current['NOUVELLE_VAR'] = 'valeur';
  store.updateProject({ envVars: current });
};

const removeEnvVar = (key: string) => {
  if (!store.currentProject) return;
  const current = { ...store.currentProject.envVars };
  delete current[key];
  store.updateProject({ envVars: current });
};

const updateEnvKey = (oldKey: string, event: any) => {
  if (!store.currentProject) return;
  const newKey = event.target.value.trim().toUpperCase();
  if (!newKey || newKey === oldKey) return;
  
  const current = { ...store.currentProject.envVars };
  const val = current[oldKey];
  delete current[oldKey];
  current[newKey] = val;
  store.updateProject({ envVars: current });
};

const updateEnvValue = (key: string, event: any) => {
  if (!store.currentProject) return;
  const newVal = event.target.value;
  const current = { ...store.currentProject.envVars };
  current[key] = newVal;
  store.updateProject({ envVars: current });
};

onMounted(async () => {
  await fetchProjectDirs();
  const loaded = localStorage.getItem('loadedConfig');
  if (loaded) {
    try {
      const config = JSON.parse(loaded);
      settings.exeName = config.exeName || 'MonApp';
      settings.phpVersion = config.phpVersion || '8.2';
      settings.port = config.port || 8080;
      settings.encryption = config.encryption || false;
      settings.compressionLevel = config.compressionLevel || 3;
      localStorage.removeItem('loadedConfig');
    } catch(e) {}
  }
});

import { dialog } from '@tauri-apps/api';

const selectDir = async () => {
  const selected = await dialog.open({
    directory: true,
    multiple: false,
  });
  if (selected) {
    store.updateProject({ sourceDir: selected as string });
  }
};

const selectEntryPoint = async () => {
  if (!store.currentProject) return;
  const selected = await dialog.open({
    defaultPath: store.currentProject.sourceDir,
    filters: [{ name: 'Fichiers PHP/HTML', extensions: ['php', 'html', 'htm'] }],
    multiple: false,
  });
  if (selected) {
    const fullPath = selected as string;
    const sourceDir = store.currentProject.sourceDir;
    if (fullPath.startsWith(sourceDir)) {
      let rel = fullPath.substring(sourceDir.length);
      if (rel.startsWith('/') || rel.startsWith('\\')) rel = rel.substring(1);
      store.updateProject({ entryPoint: rel });
    } else {
      store.updateProject({ entryPoint: fullPath.split(/[\\/]/).pop() || 'index.php' });
    }
  }
};

const selectIcon = async () => {
  if (!store.currentProject) return;
  const selected = await dialog.open({
    filters: [{ name: 'Icônes Windows', extensions: ['ico'] }],
    multiple: false,
  });
  if (selected) {
    store.updateProject({ iconPath: selected as string });
  }
};

const selectInitSql = async () => {
  if (!store.currentProject?.database) return;
  const selected = await dialog.open({
    filters: [{ name: 'Fichiers SQL', extensions: ['sql'] }],
    multiple: false,
  });
  if (selected) {
    store.currentProject.database.initSqlPath = selected as string;
    store.updateProject({ database: store.currentProject.database });
  }
};

const save = async () => {
  if (!store.currentProject) return;

  const savePath = await dialog.save({
    title: 'Enregistrer le projet sous...',
    defaultPath: `${settings.exeName}.exeoutput`,
    filters: [{ name: 'Projet ExeOutput', extensions: ['exeoutput', 'json'] }]
  });

  if (!savePath) return;

  const configToSave = {
    ...settings,
    sourceDir: store.currentProject.sourceDir,
  };

  try {
    await invoke('save_config', {
      path: savePath,
      config: configToSave,
    });
    compilerStore.addLog('info', 'Projet enregistré avec succès !');
    alert('Projet enregistré !');
  } catch (e) {
    compilerStore.addLog('error', `Erreur lors de l'enregistrement : ${e}`);
    alert(`Erreur : ${e}`);
  }
};
</script>

<style scoped>
.compiler-config {
  background: var(--bg-card);
  padding: 2rem;
  border-radius: 1rem;
  border: 1px solid var(--border);
}

.tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 2rem;
  border-bottom: 1px solid var(--border);
  padding-bottom: 1rem;
}

.tabs button {
  background: transparent;
  border: none;
  color: var(--text-muted);
  padding: 0.5rem 1rem;
  font-weight: 500;
}

.tabs button.active {
  color: var(--accent);
  border-bottom: 2px solid var(--accent);
  border-radius: 0;
}

.files-selector h3 {
  margin-bottom: 0.5rem;
}

.description {
  color: var(--text-muted);
  font-size: 0.85rem;
  margin-bottom: 1.5rem;
}

.dirs-list {
  background: var(--bg-dark);
  border-radius: 0.5rem;
  border: 1px solid var(--border);
  max-height: 300px;
  overflow-y: auto;
}

.dir-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--border);
}

.dir-item:last-child {
  border-bottom: none;
}

.dir-name {
  font-weight: 500;
}

.dir-status {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
}

.dir-status.external {
  color: var(--warning);
}

.checkbox-container {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
}

.tab-content {
  margin-bottom: 2rem;
  min-height: 200px;
}

.form-group {
  margin-bottom: 1.5rem;
}

label {
  display: block;
  margin-bottom: 0.5rem;
  color: var(--text-muted);
  font-size: 0.875rem;
}

input[type="text"],
input[type="number"],
select {
  width: 100%;
  padding: 0.75rem;
  background: var(--bg-dark);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text-main);
}

.env-var-row {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  margin-bottom: 0.5rem;
}

.env-var-row input {
  flex: 1;
}

.remove-btn {
  background: var(--error) !important;
  width: auto !important;
  padding: 0.5rem 1rem !important;
}

.add-btn {
  background: var(--bg-dark);
  border: 1px dashed var(--border);
  color: var(--text-muted);
  width: 100%;
  padding: 0.75rem;
  margin-top: 1rem;
}

.save-btn {
  width: 100%;
  padding: 1rem;
  background: var(--accent);
  color: white;
  border: none;
  font-weight: 600;
  margin-top: 2rem;
}
</style>
