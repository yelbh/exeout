# ExeOutput Studio

Studio pour la création d'applications desktop basées sur PHP.

## Nouveautés
- **Configuration Multi-Postes** : Appuyez sur `Ctrl+Shift+S` dans vos applications générées pour modifier le `.env` de la station de travail.
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
