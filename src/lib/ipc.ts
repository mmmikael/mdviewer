// Typed wrappers around the Rust commands and events (see src-tauri/src/commands.rs).

import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export interface Doc {
  path: string;
  name: string;
  dir: string;
  content: string;
  lossy: boolean;
}

export type ThemePref = "system" | "light" | "dark";
export type ContentWidth = "narrow" | "normal" | "wide" | "full";

export interface Settings {
  theme: ThemePref;
  zoom: number;
  contentWidth: ContentWidth;
  showSidebar: boolean;
  blockRemoteImages: boolean;
  editorApp: string | null;
}

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  zoom: 1,
  contentWidth: "normal",
  showSidebar: true,
  blockRemoteImages: false,
  editorApp: null,
};

export interface DefaultViewer {
  isDefault: boolean;
  /** The app that opens Markdown files now, e.g. "Xcode". */
  currentApp: string | null;
  /** False in `tauri dev`, where the app isn't a bundle macOS can register. */
  canChange: boolean;
}

export type MenuAction =
  | "find"
  | "find-next"
  | "find-prev"
  | "reload"
  | "back"
  | "forward"
  | "toggle-sidebar"
  | "zoom-reset"
  | "zoom-in"
  | "zoom-out";

export interface Events {
  /** A file was assigned to this window; fetch it with `takePendingOpen`. */
  "doc:open": null;
  "doc:changed": Doc;
  "doc:missing": string;
  "menu:action": MenuAction;
  "settings:changed": Settings;
  "recent:changed": null;
}

export const ipc = {
  showWindow: () => invoke<void>("show_window"),
  takePendingOpen: () => invoke<string | null>("take_pending_open"),
  openDocument: (path: string) => invoke<Doc>("open_document", { path }),
  reloadDocument: () => invoke<Doc>("reload_document"),
  openPaths: (paths: string[]) => invoke<void>("open_paths", { paths }),
  showOpenDialog: () => invoke<void>("show_open_dialog"),
  openExternalFile: (path: string) => invoke<void>("open_external_file", { path }),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  getRecent: () => invoke<string[]>("get_recent"),
  removeRecent: (path: string) => invoke<void>("remove_recent", { path }),
  clearRecent: () => invoke<void>("clear_recent"),
  showSettings: () => invoke<void>("show_settings"),
  chooseEditorApp: () => invoke<string | null>("choose_editor_app"),
  getDefaultViewer: () => invoke<DefaultViewer>("get_default_viewer"),
  makeDefaultViewer: () => invoke<DefaultViewer>("make_default_viewer"),
};

/**
 * Listens for an event sent to this window (or broadcast to all windows).
 * The global `listen` would also receive events targeted at *other* windows.
 */
export function on<K extends keyof Events>(
  event: K,
  handler: (payload: Events[K]) => void,
): Promise<UnlistenFn> {
  return getCurrentWebviewWindow().listen<Events[K]>(event, (e) => handler(e.payload));
}
