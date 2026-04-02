#!/bin/bash
# ExeOutput Release Script

echo "Démarrage du processus de release..."

# 1. Tests
npm run test
cargo test --manifest-path src-tauri/Cargo.toml

# 2. Build
npm run tauri:build

# 3. Signature (placeholder)
# signtool sign /f mycert.pfx /p password src-tauri/target/release/bundle/msi/*.msi

echo "Release terminée."
