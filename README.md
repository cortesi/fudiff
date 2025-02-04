[![Crates.io](https://img.shields.io/crates/v/fudiff.svg)](https://crates.io/crates/fudiff)
[![Docs](https://img.shields.io/docsrs/fudiff)](https://docs.rs/fudiff)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)

# FuDiff

A Rust library implementing a robust fuzzy unified diff format designed for AI-driven patching tools.

## Features

- Context-based patching without relying on line numbers
- Fuzzy matching for reliable patch application
- Clean, minimalist diff format optimized for AI interactions 
- Reversible patches - can apply and revert changes
- Extensive tests
- Optional serde support for serialization/deserialization (enable with *serde* feature)

## Usage

```rust
use fudiff::{diff, parse};

// Create a diff between two strings
let diff = diff("old content", "new content");

// Parse an existing diff
let diff = parse("@@ @@\n-old\n+new\n").unwrap();

// Apply a diff
let patched = diff.patch("old content").unwrap();

// Revert a diff
let original = diff.revert("new content").unwrap();
```

## Diff Format

The format uses context lines (prefixed with space), deletions (prefixed with
`-`), and additions (prefixed with `+`):

```diff
@@ @@
 fn compute(x: i32) -> i32 {
-    let y = x * 2;
-    println!("Value: {}", y);
+    let y = x + 10;
+    println!("Result: {}", y);
+    println!("Input was: {}", x);
     y
 }
```

The patch is located by matching the unchanged context lines rather than using
line numbers. Multiple changes are separated by hunk headers (`@@ @@`).

## License

MIT
