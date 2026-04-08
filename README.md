# ExeOutput Studio

- [x] Compilation d'applications PHP/Laravel en fichiers `.exe` uniques.
- [x] Support des domaines virtuels et ports personnalisés.
- [x] Système de mise à jour automatique intégré.
- [x] Gestion des versions PHP multiples (8.1, 8.2, 8.3).
- [x] Configuration multi-postes dynamique via `.env` et interface native (`Ctrl+Shift+S`).

## ⚠️ Limitations & Conseils Techniques

### Projets Laravel / Composer (Autoloading)
Sur Windows, l'utilisation de **Dossiers Externes** (`data/`) repose sur des jonctions de répertoires. 
> [!IMPORTANT]
> Pour les projets utilisant Composer (comme Laravel), le dossier `vendor` **ne doit pas** être marqué comme externe si le reste du code source (`app/`, `config/`, etc.) est inclus dans l'exécutable. 
> 
> **Pourquoi ?** L'autoloading PSR-4 utilise des chemins relatifs (`vendor/../../app`). Si `vendor` est une jonction vers le disque physique et `app` est dans un dossier temporaire, PHP ne pourra pas résoudre le chemin relatif.
> 
> **Solution** : Gardez toujours `vendor` à l'intérieur de l'EXE.

### Mise à jour Automatique
L'application vérifie la présence d'un fichier JSON distant. Pour tester, incrémentez la version dans `package.json` et faites un push sur `main`.

## Installation (Studio)
- **Déploiement Automatisé** : Support SFTP intégré via GitHub Actions.
environnement de développement pour compiler des applications PHP en exécutables Windows indépendants.

## Prérequis
- Node.js 18+
- Rust & Cargo
- PHP 8.1+ (pour le développement)

## Installation
```bash
npm install
```

## Développement
```bash
npm run tauri:dev
```

## Build
```bash
npm run tauri:build
```

## Fonctionnalités
- Serveur HTTP embarqué (Axum)
- Support PHP FFI
- Chiffrement des scripts (AES-256)
- Compression des ressources (zstd)
