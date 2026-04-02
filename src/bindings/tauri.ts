import { invoke } from '@tauri-apps/api/tauri';
import { Project, CompilerSettings } from '../types/project';

/**
 * Tauri Command Bindings
 */
export async function compileProject(name: string, source: string, output: string, entryPoint: string, publicDir: string): Promise<string> {
  return await invoke('compile_project', { name, source, output, entryPoint, publicDir });
}

export async function previewProject(source: string): Promise<number> {
  return await invoke('preview_project', { source });
}

export async function saveConfiguration(config: any): Promise<void> {
  return await invoke('save_config', { config });
}
