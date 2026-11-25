# Klink v1.0 — Published Accuracy Matrix & Honest Limits

Klink is 100% deterministic and offline (no AI/LLM inference). Every extracted edge carries an explicit confidence tag (`Static`, `Heuristic`, or `Unresolved`).

## Tier-1 Languages (Shipped in v1.0)

| Language | Static Import/Include Resolution | Dynamic/Reflection Flagging | Ground-Truth Fixture Recall |
| :--- | :--- | :--- | :--- |
| **C** | `#include "..."` (`Static`), `<...>` (`Heuristic`) | `dlopen`, `LoadLibrary` (`Unresolved`) | 100% on local fixture |
| **C++** | `#include "..."` (`Static`), `<...>` (`Heuristic`) | `dlopen`, `LoadLibrary` (`Unresolved`) | 100% on local fixture |
| **Rust** | `mod`, `use crate/super/self::`, `include!` | Macros flagged honestly | 100% on local fixture |
| **Go** | Single & grouped `import (...)` | Dynamic plugins out of scope | 100% on local fixture |
| **Java** | `import` & `import static` | `Class.forName` (`Unresolved`) | 100% on local fixture |
| **JavaScript** | ES `import`/`export`, CommonJS `require('...')` | Dynamic `require(expr)`, `import(expr)`, `eval` (`Unresolved`) | 100% on local fixture |
| **TypeScript** | ES `import`/`export`, dynamic `import('...')` | Dynamic `import(expr)`, `eval` (`Unresolved`) | 100% on local fixture |
| **Python** | `import`, `from ... import` (incl. relative dots) | `importlib.import_module`, `__import__`, `eval`, `exec` (`Unresolved`) | 100% on local fixture |
| **Binaries (PE/ELF/Mach-O)** | Pure-Rust `object` import/export tables | CodeView PDB / DWARF presence markers; degrades safely on truncated files | Verified on live native binaries |

## Known Fundamental Limits

- **Dynamic imports, reflection, and `eval`:** Undecidable statically; surfaced as `Unresolved` for manual review.
- **Macros & metaprogramming:** Klink analyzes source syntax rather than macro-expanded ASTs.
- **Minified/obfuscated bundles:** Automatically detected via Shannon entropy and mean line length and skipped loudly with a warning.
- **Cross-language FFI symbols:** Reliable only where explicit bindings or shared artifacts exist (`Heuristic`).
