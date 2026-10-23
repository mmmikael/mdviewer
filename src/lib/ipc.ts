// Typed wrappers around the Rust commands and events (see src-tauri/src/commands.rs).

import { Channel, invoke } from "@tauri-apps/api/core";
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

export type AiFormat = "openai" | "anthropic";

/** What the user configured for one AI provider (see src-tauri/src/ai/mod.rs). */
export interface ProviderSettings {
  model: string;
  /** Only used by local and custom providers. */
  baseUrl: string | null;
  /** Only used by the custom provider. */
  format: AiFormat | null;
  contextTokens: number | null;
  /** US dollars per token, from the model list (OpenRouter). */
  promptPrice: number | null;
  completionPrice: number | null;
}

export interface AiSettings {
  provider: string;
  providers: Record<string, ProviderSettings>;
  /** The user's summary instructions; null uses the built-in ones. */
  instructions: string | null;
  /** Cloud providers the user agreed to send documents to. */
  allowed: string[];
}

export interface Settings {
  theme: ThemePref;
  zoom: number;
  contentWidth: ContentWidth;
  showSidebar: boolean;
  blockRemoteImages: boolean;
  editorApp: string | null;
  ai: AiSettings;
}

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  zoom: 1,
  contentWidth: "normal",
  showSidebar: true,
  blockRemoteImages: false,
  editorApp: null,
  ai: { provider: "openrouter", providers: {}, instructions: null, allowed: [] },
};

export const EMPTY_PROVIDER_SETTINGS: ProviderSettings = {
  model: "",
  baseUrl: null,
  format: null,
  contextTokens: null,
  promptPrice: null,
  completionPrice: null,
};

export interface AiProvider {
  id: string;
  name: string;
  format: AiFormat;
  baseUrl: string;
  needsKey: boolean;
  /** Runs on this Mac, so documents never leave it. */
  local: boolean;
  keyUrl: string | null;
  defaultModel: string | null;
  editableUrl: boolean;
}

export interface AiModel {
  id: string;
  name: string | null;
  contextTokens: number | null;
  promptPrice: number | null;
  completionPrice: number | null;
}

export type AiMessage = { type: "text"; text: string } | { type: "notice"; text: string };

export interface DefaultViewer {
  isDefault: boolean;
  /** The app that opens Markdown files now, e.g. "Xcode". */
  currentApp: string | null;
  /** False in `tauri dev`, where the app isn't a bundle macOS can register. */
  canChange: boolean;
}

/** Where macOS put the title bar and window buttons, in points (see platform.rs). */
export interface WindowChrome {
  titlebarHeight: number;
  /** Right edge of the zoom (green) button. */
  controlsEnd: number;
  /** Vertical centre of the window buttons. */
  controlsCenterY: number;
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
  | "zoom-out"
  | "export-html"
  | "copy-rich"
  | "summarize";

export interface Events {
  /** A file was assigned to this window; fetch it with `takePendingOpen`. */
  "doc:open": null;
  "doc:changed": Doc;
  "doc:missing": string;
  "menu:action": MenuAction;
  "settings:changed": Settings;
  "recent:changed": null;
  /** A PDF export finished; the payload is the saved file. */
  "export:done": string;
  "export:failed": string;
}

export const ipc = {
  showWindow: () => invoke<void>("show_window"),
  windowChrome: () => invoke<WindowChrome | null>("window_chrome"),
  takePendingOpen: () => invoke<string | null>("take_pending_open"),
  openDocument: (path: string) => invoke<Doc>("open_document", { path }),
  reloadDocument: () => invoke<Doc>("reload_document"),
  openPaths: (paths: string[]) => invoke<void>("open_paths", { paths }),
  showOpenDialog: () => invoke<void>("show_open_dialog"),
  openExternalFile: (path: string) => invoke<void>("open_external_file", { path }),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  copyRich: (html: string, text: string) => invoke<void>("copy_rich", { html, text }),
  /** Asks where to save, then writes the file. Resolves to the path, or null if cancelled. */
  exportHtml: (html: string) => invoke<string | null>("export_html", { html }),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  getRecent: () => invoke<string[]>("get_recent"),
  removeRecent: (path: string) => invoke<void>("remove_recent", { path }),
  clearRecent: () => invoke<void>("clear_recent"),
  showSettings: () => invoke<void>("show_settings"),
  chooseEditorApp: () => invoke<string | null>("choose_editor_app"),
  getDefaultViewer: () => invoke<DefaultViewer>("get_default_viewer"),
  makeDefaultViewer: () => invoke<DefaultViewer>("make_default_viewer"),
  aiProviders: () => invoke<AiProvider[]>("ai_providers"),
  /** Whether a key is saved; the key itself never comes back to the page. */
  aiKeyStatus: (provider: string) => invoke<boolean>("ai_key_status", { provider }),
  /** Saves the key in the keychain; an empty key removes it. */
  aiSetKey: (provider: string, key: string) => invoke<void>("ai_set_key", { provider, key }),
  aiModels: (provider: string) => invoke<AiModel[]>("ai_models", { provider }),
  /** Sends a prompt to the selected provider; resolves when the reply is complete. */
  aiComplete: (id: number, system: string, prompt: string, onMessage: (message: AiMessage) => void) =>
    invoke<void>("ai_complete", { id, system, prompt, onMessage: new Channel<AiMessage>(onMessage) }),
  aiCancel: (id: number) => invoke<void>("ai_cancel", { id }),
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
