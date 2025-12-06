# Rustymix Roadmap

## Short Term
- [ ] **Optimized Token Counting**: Implement faster token counting (possibly caching results for unchanged files).
- [ ] **Smart Context Selection**: Use LLMs to intelligently select which files to include based on a high-level intent, rather than just relying on manual `--focus` or naive searching.
- [ ] **Pre-commit Hook Integration**: Easy install script to run repomix as a pre-commit hook.

## Medium Term
- [ ] **Plugin System**: Allow users to write plugins (e.g., in Lua or WASM) to custom-process files.
- [ ] **Language-Specific Enhancements**: Better compression and comment stripping for more languages (Ruby, PHP, Swift, etc.).
- [ ] **Interactive Mode**: A TUI (Terminal User Interface) to select files and set options interactively.

## Long Term
- [ ] **LLM Integration**: Directly call LLM APIs (OpenAI, Anthropic) to generate code diffs based on the repomix output.
- [ ] **Semantic Search**: Index the codebase and allow semantic searching for relevant files to include in the pack.
- [ ] **IDE Extensions**: VS Code and JetBrains plugins to generate packs directly from the editor.
