# Rustymix 🦀

**Rustymix** is a high-performance, native Rust port of [Repomix](https://github.com/yamadashy/repomix), evolved into a **Context Intelligence Engine** for LLMs.

It doesn't just pack files; it curates context. It allows you to feed LLMs a lightweight "Skeleton" of your entire repo to identify relevant files, then generates a "Hybrid" pack containing **full source code** for the files you need to edit, while keeping the rest as **compressed signatures** to prevent hallucinations.

## 🚀 Features

- **Context Intelligence**: Hybrid packing allows you to mix "Full Text" (focused files) and "Skeleton" (compressed context) in a single output.
- **Smart Prompt Injection**: Automatically attaches your intent ("Fix the login bug") and instructions to the top of the generated file.
- **Blazing Fast**: Written in Rust using Tokio and the `ripgrep` engine (`ignore`) for instant file walking.
- **Tree-sitter Compression**: Parses code to strip implementation details from non-focus files, saving 40-60% of tokens while retaining type safety.
- **Security Check**: Automatically detects and excludes suspicious secrets (API keys, tokens).
- **Git Aware**: Respects `.gitignore`, supports remote repo cloning, and includes git diffs/logs.
- **Clipboard Ready**: Copies the output directly to your system clipboard.

## 📦 Installation

### From Source
```bash
git clone [https://github.com/yourusername/rustymix](https://github.com/yourusername/rustymix)
cd rustymix
cargo install --path .
````

## 🧠 The "Smart Context" Workflow

Rustymix is designed for a 2-pass workflow that saves money and improves LLM accuracy.

### Phase 1: Survey (The "Map")

Ask the LLM what it needs to look at. Rustymix generates a **Skeleton** of your entire repo and injects a prompt asking the LLM to identify relevant files.

```bash
rustymix --compress --intent "There is a bug in the login retry logic. Which files do I need to fix it?"
```

**Output sent to LLM:**

> **System:** Attached is the SKELETON of the codebase. Return a list of file paths that must be read in full text.
> **Context:** `src/auth/login.ts` (signatures only), `src/utils/retry.ts` (signatures only)...

**LLM Response:**

> "I need to see `src/auth/login.ts` and `src/utils/retry.ts`."

-----

### Phase 2: Build (The "Pack")

Feed the LLM exactly what it asked for. Use the `--focus` flag to give it full access to specific files while keeping the rest of the repo as a skeleton for context.

```bash
rustymix --focus "src/auth/login.ts,src/utils/retry.ts" --compress --intent "Fix the retry logic bug."
```

**Output sent to LLM:**

> **System:** Attached is the CONTEXT PACK. Files marked `mode="full"` are editable. Files marked `mode="skeleton"` are read-only context.
> **File 1:** `src/auth/login.ts` (FULL TEXT)
> **File 2:** `src/utils/retry.ts` (FULL TEXT)
> **File 3:** `src/database/db.ts` (SKELETON - Context only)

## 🛠 Usage

### Common Commands

```bash
# Standard pack (everything in full text)
rustymix

# Output to Markdown and copy to clipboard
rustymix --style markdown --copy

# Process a remote repository
rustymix --remote [https://github.com/yamadashy/repomix](https://github.com/yamadashy/repomix) --output repomix-source.xml
```

### All Flags

| Flag | Description |
|------|-------------|
| `--intent <TEXT>` | **(New)** Injects your natural language task at the top of the file to guide the LLM. |
| `--focus <FILES>` | **(New)** Comma-separated list of files to include in **Full Text**. All other files respect the `--compress` flag. |
| `--compress` | Uses Tree-sitter to strip implementation details from files not in `--focus`. |
| `-o, --output <FILE>` | Output file path (default: `repomix-output.xml`). |
| `--style <STYLE>` | Output style: `xml`, `markdown`, `json`, `plain`. |
| `--copy` | Copy output to system clipboard. |
| `--remote <URL>` | Process a remote Git repository. |
| `--security-check <BOOL>` | Enable/Disable secret scanning (default: true). |
| `--include <PATTERN>` | Comma-separated glob patterns to include. |
| `--ignore <PATTERN>` | Comma-separated glob patterns to ignore. |
| `--include-diffs` | Include `git diff` (staged and unstaged) in output. |
| `--include-logs` | Include recent `git log` in output. |

## ⚙️ Configuration

Rustymix automatically detects `repomix.config.json` in your project root.

```json
{
  "output": {
    "style": "markdown",
    "removeComments": true,
    "compress": true,
    "topFilesLength": 10
  },
  "ignore": {
    "customPatterns": ["**/*.test.ts", "legacy/**"]
  },
  "security": {
    "enableSecurityCheck": true
  }
}
```

*Note: `--focus` and `--intent` are currently CLI-only arguments to ensure they are specific to the current task.*

## 🆚 Comparison

| Feature | Original Repomix (Node) | Rustymix (Rust) |
|---------|-------------------------|-----------------|
| **Philosophy** | "Pack everything" | "Smart Context" |
| **Hybrid Packing** | No | **Yes** (Focus + Compress) |
| **Prompt Injection**| No | **Yes** (via `--intent`) |
| **Runtime** | Node.js | Native Binary |
| **Startup Speed** | Slow (\~500ms+) | Instant (\~10ms) |
| **Parser** | Tree-sitter (WASM) | Tree-sitter (Native C) |

## 📄 License

MIT
