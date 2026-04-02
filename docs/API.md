# API Documentation

## Tauri Commands

### `compile_project`
Compile un dossier source PHP en un fichier EXE.
- **Paramètres**: `name`, `source`, `output`
- **Retour**: message de succès ou erreur.

### `preview_project`
Lance un serveur de prévisualisation local.
- **Paramètres**: `source`
- **Retour**: port du serveur.

### `save_config`
Sauvegarde la configuration du projet au format JSON.
