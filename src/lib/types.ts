export interface Service {
  id: string;
  name: string;
  url: string;
  groupId: string | null;
  icon: string | null;
  zoom: number;
  order: number;
}

export interface Group {
  id: string;
  name: string;
  order: number;
  collapsed: boolean;
}

export interface Profile {
  id: string;
  name: string;
  updatedAt: number;
  services: Service[];
  groups: Group[];
}

export interface Prefs {
  theme: "system" | "dark" | "light";
  suspendMinutes: number;
}

export interface AppData {
  version: number;
  rev: number;
  activeProfileId: string;
  profiles: Record<string, Profile>;
  prefs: Prefs;
}

export interface WebdavSettings {
  url: string;
  username: string;
  remotePath: string;
  autoSync: boolean;
}

export interface LocalSettings {
  closeToTray: "tray" | "minimize" | "quit";
  startMinimized: boolean;
  globalShortcut: string;
  autostart: boolean;
  ignoreCertErrors: boolean;
  webdav: WebdavSettings;
  syncMeta: {
    lastSyncedRev: number;
    lastSyncedAt: number;
  };
}

export interface AppState {
  data: AppData;
  settings: LocalSettings;
  hasWebdavPassword: boolean;
}

export interface SyncOutcome {
  status: string;
  localRev: number;
  remoteRev: number;
  message: string | null;
}

export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}
