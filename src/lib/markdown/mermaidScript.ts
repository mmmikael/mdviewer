// Loads Mermaid's single-file build (`mermaid.min.js`) with a classic script
// tag. Used by the app and by Quick Look, which share one copy of the file.

export type Mermaid = (typeof import("mermaid"))["default"];

let loading: Promise<Mermaid> | null = null;

export function loadMermaidScript(src: string): Promise<Mermaid> {
  loading ??= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = src;
    script.onload = () => resolve((globalThis as unknown as { mermaid: Mermaid }).mermaid);
    script.onerror = () => {
      loading = null;
      reject(new Error("Couldn’t load the diagram renderer."));
    };
    document.head.append(script);
  });
  return loading;
}
