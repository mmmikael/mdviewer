// Lets the UI run in a plain browser (http://127.0.0.1:1420/?file=rich.md) for
// quick UI work and browser-driven tests. Only loaded in dev outside Tauri.
//
// Query parameters: `file` (rich.md, other.md, Help.md) opens a fixture,
// `window=settings` shows the Settings window. `window.mdv.menu("find")`
// triggers a menu action from the console.

import type { Channel } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import help from "../../src-tauri/resources/Help.md?raw";
import other from "../../tests/fixtures/other.md?raw";
import rich from "../../tests/fixtures/rich.md?raw";
import {
  DEFAULT_SETTINGS,
  type AiMessage,
  type AiModel,
  type AiProvider,
  type Doc,
  type MenuAction,
  type Settings,
} from "../lib/ipc";

const DIR = "/mock";
const files: Record<string, string> = {
  [`${DIR}/Help.md`]: help,
  [`${DIR}/rich.md`]: rich,
  [`${DIR}/other.md`]: other,
};

const providers: AiProvider[] = [
  {
    id: "openrouter",
    name: "OpenRouter",
    format: "openai",
    baseUrl: "https://openrouter.ai/api/v1",
    needsKey: true,
    local: false,
    keyUrl: "https://openrouter.ai/settings/keys",
    defaultModel: null,
    editableUrl: false,
  },
  {
    id: "anthropic",
    name: "Anthropic",
    format: "anthropic",
    baseUrl: "https://api.anthropic.com/v1",
    needsKey: true,
    local: false,
    keyUrl: "https://platform.claude.com/settings/keys",
    defaultModel: "claude-opus-5-5",
    editableUrl: false,
  },
  {
    id: "ollama",
    name: "Ollama",
    format: "openai",
    baseUrl: "http://localhost:11434/v1",
    needsKey: false,
    local: true,
    keyUrl: null,
    defaultModel: null,
    editableUrl: true,
  },
  {
    id: "custom",
    name: "Custom",
    format: "openai",
    baseUrl: "",
    needsKey: false,
    local: false,
    keyUrl: null,
    defaultModel: null,
    editableUrl: true,
  },
];

const models: AiModel[] = [
  { id: "mock/small", name: "Mock Small", contextTokens: 8192, promptPrice: 0, completionPrice: 0 },
  { id: "mock/large", name: "Mock Large", contextTokens: 200000, promptPrice: 0.000003, completionPrice: 0.000015 },
];

const DIAGRAM = `\`\`\`mermaid
flowchart LR
  A["Markdown file"] -->|"saved"| B["MDViewer"]
  B --> C["Rendered page"]
  B --> D["TL;DR card"]
%% link: #rich-content Rendered page
\`\`\`

**Sections:** [Rich content](#rich-content)`;

// `?diagram=mindmap` answers with a mind map instead.
const MINDMAP = `\`\`\`mermaid
mindmap
  root((Rich content))
    Rendering
      Code and math
      Diagrams
    Live reload
%% link: #rich-content Code and math
%% link: #rich-content Diagrams
\`\`\`

**Sections:** [Rich content](#rich-content)`;

// What a model might get wrong: unquoted parentheses in a label.
const BAD_DIAGRAM = "```mermaid\nflowchart LR\n  A[Broken (label] --> B\n```";

const SUMMARY = `- A **sample document** that shows what MDViewer renders: [tables](#rich-content), code, math and diagrams.
- Live reload keeps the scroll position when the file changes.
- Open question: whether to support [footnotes](#rich-content) in exports.`;

function doc(path: string): Doc {
  if (!(path in files)) throw `Couldn’t open “${path}”: No such file or directory`;
  return { path, name: path.slice(DIR.length + 1), dir: DIR, content: files[path], lossy: false };
}

export function installMocks() {
  const params = new URLSearchParams(location.search);
  const file = params.get("file");
  let settings: Settings = { ...DEFAULT_SETTINGS };
  let current: string | null = null;
  let isDefault = false;
  let recent = [...Object.keys(files), "/elsewhere/notes.md", "/elsewhere/todo.md"];
  const keys = new Set<string>();
  const cancelled = new Set<number>();

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
          return recent;
        case "remove_recent":
          recent = recent.filter((path) => path !== a.path);
          void emit("recent:changed", null);
          return null;
        case "clear_recent":
          recent = [];
          void emit("recent:changed", null);
          return null;
        case "export_html":
          (window as unknown as { mdvLastExport: unknown }).mdvLastExport = a.html;
          return `${DIR}/${current?.slice(DIR.length + 1).replace(/\.[^.]+$/, "")}.html`;
        case "copy_rich":
          (window as unknown as { mdvLastClipboard: unknown }).mdvLastClipboard = { html: a.html, text: a.text };
          return null;
        case "window_chrome":
          return { titlebarHeight: 66, controlsEnd: 79, controlsCenterY: 26 };
        case "get_default_viewer":
          return { isDefault, currentApp: isDefault ? "MDViewer" : "Xcode", canChange: true };
        case "make_default_viewer":
          isDefault = true;
          return { isDefault, currentApp: "MDViewer", canChange: true };
        // AI summaries: `?ai=fail` makes requests fail.
        case "ai_providers":
          return providers;
        case "ai_key_status":
          return keys.has(a.provider as string);
        case "ai_set_key":
          if (a.key) keys.add(a.provider as string);
          else keys.delete(a.provider as string);
          return null;
        case "ai_models":
          return models;
        case "ai_cancel":
          cancelled.add(a.id as number);
          return null;
        case "ai_complete":
          return (async () => {
            if (params.get("ai") === "fail") throw "OpenRouter didn’t accept the API key. Check it in Settings.";
            const channel = a.onMessage as Channel<AiMessage>;
            // `?ai=baddiagram` makes the first diagram invalid, to try the repair.
            const diagram = String(a.system).includes("Mermaid diagram");
            const repairing = String(a.prompt).includes("couldn't draw");
            const good = params.get("diagram") === "mindmap" ? MINDMAP : DIAGRAM;
            const reply = !diagram ? SUMMARY : params.get("ai") === "baddiagram" && !repairing ? BAD_DIAGRAM : good;
            for (const word of reply.split(/(?<= )/)) {
              await new Promise((resolve) => setTimeout(resolve, 15));
              if (cancelled.has(a.id as number)) throw "Cancelled.";
              channel.onmessage({ type: "text", text: word });
            }
            return null;
          })();
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
