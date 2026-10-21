// Lets the UI run in a plain browser (http://127.0.0.1:1420/?file=rich.md) for
// quick UI work and browser-driven tests. Only loaded in dev outside Tauri.
//
// Query parameters: `file` (rich.md, other.md, Help.md) opens a fixture,
// `window=settings` shows the Settings window. `window.mdv.menu("find")`
// triggers a menu action from the console.

import { emit } from "@tauri-apps/api/event";
import { mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import help from "../../src-tauri/resources/Help.md?raw";
import other from "../../tests/fixtures/other.md?raw";
import rich from "../../tests/fixtures/rich.md?raw";
import { DEFAULT_SETTINGS, type Doc, type MenuAction, type Settings } from "../lib/ipc";

const DIR = "/mock";
const files: Record<string, string> = {
  [`${DIR}/Help.md`]: help,
  [`${DIR}/rich.md`]: rich,
  [`${DIR}/other.md`]: other,
};

function doc(path: string): Doc {
  if (!(path in files)) throw `Couldn’t open “${path}”: No such file or directory`;
  return { path, name: path.slice(DIR.length + 1), dir: DIR, content: files[path], lossy: false };
}

export function installMocks() {
  const params = new URLSearchParams(location.search);
  const file = params.get("file");
  let settings: Settings = { ...DEFAULT_SETTINGS };
  let current: string | null = null;

  mockWindows(params.get("window") ?? "doc-1");
  mockConvertFileSrc("macos");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "take_pending_open":
          return file ? `${DIR}/${file}` : null;
        case "open_document":
          current = a.path as string;
          return doc(current);
        case "reload_document":
          return doc(current!);
        case "get_settings":
          return settings;
        case "set_settings":
          settings = a.settings as Settings;
          void emit("settings:changed", settings);
          return null;
        case "get_recent":
          return Object.keys(files);
        default:
          console.info("[mock ipc]", cmd, args);
          return null;
      }
    },
    { shouldMockEvents: true },
  );

  Object.assign(window, {
    mdv: {
      menu: (action: MenuAction) => emit("menu:action", action),
      edit: (content: string) => current && emit("doc:changed", { ...doc(current), content }),
    },
  });
}
