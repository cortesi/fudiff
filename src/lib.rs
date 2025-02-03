//! Implementation of the Fuzzy Unified Diff Format
//!
//! # Implementation Guide
//!
//! ## Parsing
//! Use a line-by-line parser that distinguishes file header lines and hunk sections.
//! A state machine approach is recommended:
//!
//! 1. Read file headers
//! 2. On encountering `@@ @@`, begin a new hunk
//! 3. Process subsequent hunk body lines until the next hunk header or end-of-file
//!
//! ## Fuzzy Matching
//! Implement a fuzzy matching algorithm to locate the context block in the target file.
//! The [`fuzzy-matcher`](https://crates.io/crates/fuzzy-matcher) crate may be useful.
//! Normalize whitespace and trailing newlines to reduce false mismatches.
//!
//! ## Error Reporting
//! Provide clear error messages for ambiguous contexts, incomplete hunks, or parsing
//! failures to assist debugging and user feedback.
//!
//! ## Testing
//! Write comprehensive tests for all corner cases, ensuring that the patch application
//! logic handles various diff scenarios robustly.

/// Core error types for the diff parser and patcher
#[derive(Debug)]
pub enum Error {
    /// Failed to parse the diff format
    Parse(String),
    /// Failed to apply the patch
    Apply(String),
    /// Multiple possible matches found for context
    AmbiguousMatch(String),
}

/// Result type alias for diff operations
pub type Result<T> = std::result::Result<T, Error>;

/// Represents a complete fuzzy diff
pub struct FuzzyDiff {
    pub original_file: String,
    pub modified_file: String,
    pub hunks: Vec<Hunk>,
}

/// Represents a single hunk within a diff
pub struct Hunk {
    pub context_before: Vec<String>,
    pub deletions: Vec<String>,
    pub additions: Vec<String>,
    pub context_after: Vec<String>,
}
