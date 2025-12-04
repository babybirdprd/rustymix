# Rustymix 🦀

**Rustymix** is a high-performance, native Rust port of [Repomix](https://github.com/yamadashy/repomix). It packs your entire repository (or specific subsets) into a single, AI-friendly file (XML, Markdown, JSON, or Plain Text).

It is designed to be **fast**, **dependency-free** (no Node.js required), and **100% compatible** with Repomix configuration files.

## 🚀 Features

- **Blazing Fast**: Written in Rust using Tokio for concurrency and `ignore` (the engine behind `ripgrep`) for file walking.
- **Smart Compression**: Uses **Tree-sitter** to parse code and extract only essential signatures (classes, functions, interfaces) when using `--compress`.
- **Token Counting**: Built-in `tiktoken` support to count tokens for LLM context windows.
- **Security Check**: Automatically detects and excludes suspicious secrets (API keys, tokens).
- **Git Aware**: Respects `.gitignore`, supports remote repo cloning, and can include git diffs/logs.
- **Clipboard Ready**: Copies the output directly to your clipboard.

## 📦 Installation

### From Source
```bash
git clone https://github.com/yourusername/rustymix
cd rustymix
cargo install --path .
```

### Pre-built Binaries
*(Coming soon via GitHub Releases)*

## 🛠 Usage

Run it in your project root:

```bash
rustymix
```

This generates `repomix-output.xml` by default.

### Common Options

```bash
# Output to Markdown and copy to clipboard
rustymix --style markdown --copy

# Compress code (extract signatures only) to save context
rustymix --compress

# Process a remote repository
rustymix --remote https://github.com/yamadashy/repomix --output repomix-source.xml

# Remove comments and empty lines to save tokens
rustymix --remove-comments --remove-empty-lines
```

### All Flags

| Flag | Description |
|------|-------------|
| `-o, --output <FILE>` | Output file path (default: `repomix-output.xml`) |
| `--style <STYLE>` | Output style: `xml`, `markdown`, `json`, `plain` |
| `--compress` | Use Tree-sitter to strip implementation details, keeping only signatures |
| `--copy` | Copy output to system clipboard |
| `--remote <URL>` | Process a remote Git repository |
| `--security-check <BOOL>` | Enable/Disable secret scanning (default: true) |
| `--include <PATTERN>` | Comma-separated glob patterns to include |
| `--ignore <PATTERN>` | Comma-separated glob patterns to ignore |
| `--include-diffs` | Include `git diff` (staged and unstaged) in output |
| `--include-logs` | Include recent `git log` in output |

## ⚙️ Configuration

Rustymix automatically detects `repomix.config.json` in your project root. It is fully compatible with the original Repomix schema.

**Example `repomix.config.json`:**

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

## 🧠 How Compression Works

When you pass the `--compress` flag, Rustymix uses **Tree-sitter** (a native parsing library) to understand the syntax of your code.

Instead of including the full function bodies, it extracts the "skeleton" of your code:
- Class definitions
- Function signatures
- Interface definitions
- Structs and Enums

**Supported Languages for Compression:**
- Rust (`.rs`)
- TypeScript / JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`)
- Python (`.py`)
- Go (`.go`)

*Other languages will be included in full text if compression is enabled but the language is not supported yet.*

## 🆚 Comparison

| Feature | Original Repomix (Node) | Rustymix (Rust) |
|---------|-------------------------|-----------------|
| **Runtime** | Node.js | Native Binary |
| **Startup Speed** | Slow (~500ms+) | Instant (~10ms) |
| **File Walking** | Node `fs` | `ripgrep` engine |
| **Parser** | Tree-sitter (WASM) | Tree-sitter (Native C) |
| **Single File** | No (requires npm) | Yes |

## 📄 License

MIT
```

### 3. Build Instructions

Since we are linking C libraries (Tree-sitter), you need a C compiler installed on your system.

**Linux (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install build-essential
```

**macOS:**
```bash
xcode-select --install
```

**Windows:**
Install Visual Studio Build Tools (C++ workload).

Then simply run:
```bash
cargo build --release
```

The binary will be located at `./target/release/rustymix`.
