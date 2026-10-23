# Contributing to MDViewer

Thanks for helping! Bug reports, ideas and pull requests are all welcome.

## Issues

- **Bugs:** use the bug report form and include your MDViewer and macOS versions. For rendering problems, the smallest piece of Markdown that shows it helps most.
- **Ideas:** use the feature request form and start with the problem you're trying to solve. MDViewer is meant to stay a small, fast viewer rather than become an editor, so not every idea will fit, but the reasons behind it often lead somewhere useful.
- **Security issues:** please don't post the details publicly. Open an issue that only says you found a security problem and how to reach you, and we'll take it from there privately.

## Pull requests

For anything bigger than a small fix, open an issue first so we can agree on the approach before you spend time on it.

1. Fork the repository and create a branch from `main`.
2. Set up and run the app (needs Rust 1.89+, Node 22+, pnpm and the Xcode command line tools):

   ```bash
   pnpm install
   pnpm tauri dev
   ```

   Most UI work can also be done in a browser at <http://127.0.0.1:1420/?file=rich.md> while `pnpm dev` runs. The [README's Development section](README.md#development) has more.
3. Before you push, make sure these pass (CI runs them too):

   ```bash
   pnpm lint
   ```

   ```bash
   pnpm test
   ```

   ```bash
   cargo test --manifest-path src-tauri/Cargo.toml
   ```

4. Add or update tests for what you changed, and keep the pull request to one topic.
5. If the change is visible, describe it in the pull request and add a screenshot.

## Code style

- Match the code around your change: naming, comment style and structure.
- The web UI is Svelte 5 with TypeScript; the native side is Rust. Formatting is checked by `cargo fmt` and `svelte-check`.
- Documents are untrusted input: rendered HTML goes through DOMPurify, and the page only reaches files through the app's own commands. Keep it that way.
- AI providers live in `src-tauri/src/ai/providers.rs`. A new OpenAI-compatible provider is usually one entry in that list.

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
