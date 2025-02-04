# FuDiff

A Rust library implementing a robust fuzzy unified diff format designed for AI-driven patching tools.

## Features

- Context-based patching without relying on line numbers
- Fuzzy matching for reliable patch application
- Clean, minimalist diff format optimized for AI interactions 
- Reversible patches - can apply and revert changes
- Extensive tests

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

## License

MIT
