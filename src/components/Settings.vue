<template>
  <div class="settings-container">
    <h2>Paramètres Globaux</h2>
    <div class="form-group">
      <label>Chemin PHP par défaut</label>
      <input v-model="globalSettings.phpPath" type="text" />
    </div>
    <div class="form-group">
      <label>Thème de l'interface</label>
      <select v-model="globalSettings.theme">
        <option value="light">Clair</option>
        <option value="dark">Sombre (Conseillé)</option>
      </select>
    </div>
    <button class="primary" style="margin-top: 2rem">Enregistrer les paramètres</button>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue';

const globalSettings = reactive({
  phpPath: 'C:\\php\\php.exe',
  theme: 'dark',
});
</script>

<style scoped>
.settings-container {
  background: var(--bg-card);
  padding: 2.5rem;
  border-radius: 1rem;
  border: 1px solid var(--border);
  max-width: 600px;
}

h2 {
  margin-top: 0;
  margin-bottom: 2rem;
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

input, select {
  width: 100%;
  padding: 0.75rem;
  background: var(--bg-dark);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text-main);
}
</style>
