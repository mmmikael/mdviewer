import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";

import "./styles/app.css";
import DocumentWindow from "./DocumentWindow.svelte";
import SettingsWindow from "./SettingsWindow.svelte";

async function start() {
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMocks } = await import("./dev/mockTauri");
    installMocks();
  }

  const target = document.getElementById("app")!;
  const isSettings = getCurrentWindow().label === "settings";
  document.body.classList.add(isSettings ? "settings-window" : "document-window");
  mount(isSettings ? SettingsWindow : DocumentWindow, { target });
}

void start();
