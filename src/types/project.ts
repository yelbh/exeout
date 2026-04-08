export interface Project {
  name: string;
  sourceDir: string;
  version: string;
  icon?: string;
  entryPoint: string;
  publicDir: string;
  externalDirs: string[];
  iconPath?: string;
  database?: DatabaseConfig;
  updateUrl?: string;
  notes?: string;
  server?: ServerConfig;
  envVars: Record<string, string>;
}

export interface ServerConfig {
  host: string;
  user: string;
  pass: string;
  port: number;
  remotePath: string;
}

export interface DatabaseConfig {
  type: 'none' | 'sqlite' | 'mariadb';
  port?: number;
  databaseName?: string;
  username?: string;
  password?: string;
  initSqlPath?: string;
}

export interface CompilerSettings {
  phpVersion: string;
  extensions: string[];
  port: number;
  timeout: number;
  encryption: boolean;
  compressionLevel: number;
  externalDirs: string[];
}

export interface LogEntry {
  level: 'info' | 'warning' | 'error';
  message: string;
  timestamp: number;
}
