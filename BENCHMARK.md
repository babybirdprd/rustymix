# Benchmark Results

Comparison between `rustymix` (native binary) and `repomix` (Node.js/npx).
Measured 3 iterations per repository.

| Repository | Rustymix (s) | Repomix (s) | Speedup |
|---|---|---|---|
| Rustymix (Self) | **0.7658s** | 4.4625s | **5.83x** |
| Express (Node Repo) | **6.3862s** | 4.6462s | **0.73x** |
| Tokio (Rust Repo) | **23.8623s** | 13.1039s | **0.55x** |
