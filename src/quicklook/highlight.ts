// Quick Look's highlighter: a fixed set of common languages compiled into the
// preview, because the extension's script must be a single file (no lazy
// loading over file://). Swapped in for src/lib/markdown/highlight.ts by
// quicklook/vite.config.ts.

import c from "shiki/langs/c.mjs";
import csharp from "shiki/langs/csharp.mjs";
import css from "shiki/langs/css.mjs";
import diff from "shiki/langs/diff.mjs";
import docker from "shiki/langs/docker.mjs";
import go from "shiki/langs/go.mjs";
import html from "shiki/langs/html.mjs";
import ini from "shiki/langs/ini.mjs";
import java from "shiki/langs/java.mjs";
import javascript from "shiki/langs/javascript.mjs";
import json from "shiki/langs/json.mjs";
import kotlin from "shiki/langs/kotlin.mjs";
import lua from "shiki/langs/lua.mjs";
import markdown from "shiki/langs/markdown.mjs";
import python from "shiki/langs/python.mjs";
import rust from "shiki/langs/rust.mjs";
import scss from "shiki/langs/scss.mjs";
import shellscript from "shiki/langs/shellscript.mjs";
import sql from "shiki/langs/sql.mjs";
import swift from "shiki/langs/swift.mjs";
import toml from "shiki/langs/toml.mjs";
import tsx from "shiki/langs/tsx.mjs";
import typescript from "shiki/langs/typescript.mjs";
import xml from "shiki/langs/xml.mjs";
import yaml from "shiki/langs/yaml.mjs";
import githubDark from "shiki/themes/github-dark.mjs";
import githubLight from "shiki/themes/github-light.mjs";

import { createHighlighting } from "../lib/markdown/highlightCore";

export { fenceLanguages } from "../lib/markdown/highlightCore";

export const { prepareHighlighting, highlight } = createHighlighting({
  themes: [githubLight, githubDark],
  langs: [
    c, csharp, css, diff, docker, go, html, ini, java, javascript, json, kotlin, lua,
    markdown, python, rust, scss, shellscript, sql, swift, toml, tsx, typescript, xml, yaml,
  ],
});
