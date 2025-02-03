//! Implementation of the Fuzzy Unified Diff Format

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
/// Represents a complete fuzzy diff
#[derive(Debug)]
pub struct FuDiff {
    pub hunks: Vec<Hunk>,
}

impl FuDiff {
    /// Parse a fuzzy diff from a string.
    pub fn parse(input: &str) -> Result<Self> {
        let mut hunks = Vec::new();
        let mut current_hunk: Option<Hunk> = None;

        // No hunk headers found in input
        if !input.contains("@@") {
            return Err(Error::Parse("No hunks found in diff".to_string()));
        }

        for line in input.lines() {
            if line.starts_with("@@") {
                // When we see a new hunk header, push any existing hunk
                if let Some(hunk) = current_hunk.take() {
                    hunks.push(hunk);
                }
                current_hunk = Some(Hunk {
                    context_before: Vec::new(),
                    deletions: Vec::new(),
                    additions: Vec::new(),
                    context_after: Vec::new(),
                });
                continue;
            }

            // Skip empty lines and file headers
            if line.is_empty() || line.starts_with("---") || line.starts_with("+++") {
                continue;
            }

            // If we haven't seen a hunk header yet and get non-header content, it's an error
            if current_hunk.is_none() && !line.starts_with("@@") {
                return Err(Error::Parse("Line found outside of hunk".to_string()));
            }

            // Past this point, we must have a hunk for non-header lines
            if !line.starts_with("@@") {
                let hunk = current_hunk
                    .as_mut()
                    .expect("Internal error: hunk should exist");

                // First character determines line type
                let (marker, content) = line.split_at(1);
                match marker {
                    " " => {
                        // Context lines go into before/after based on whether we've seen changes
                        if hunk.deletions.is_empty() && hunk.additions.is_empty() {
                            hunk.context_before.push(content.to_string());
                        } else {
                            hunk.context_after.push(content.to_string());
                        }
                    }
                    "-" => hunk.deletions.push(content.to_string()),
                    "+" => hunk.additions.push(content.to_string()),
                    _ => return Err(Error::Parse(format!("Invalid line prefix: {}", marker))),
                }
            }
        }

        // Don't forget the last hunk
        if let Some(ref hunk) = current_hunk {
            hunks.push(hunk.clone());
        }

        // If we have no hunks after processing all input, we never saw a hunk header
        if hunks.is_empty() {
            return Err(Error::Parse("No hunks found in diff".to_string()));
        }

        Ok(FuDiff { hunks })
    }
}

/// Represents a single hunk within a diff
#[derive(Debug, Clone)]
pub struct Hunk {
    pub context_before: Vec<String>,
    pub deletions: Vec<String>,
    pub additions: Vec<String>,
    pub context_after: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_hunk() -> Result<()> {
        let diff = "\
@@ @@
 fn main() {
-    println!(\"Hello\");
+    println!(\"Goodbye\");
 }";
        let fudiff = FuDiff::parse(diff)?;
        assert_eq!(fudiff.hunks.len(), 1);
        let hunk = &fudiff.hunks[0];
        assert_eq!(hunk.context_before, vec!["fn main() {"]);
        assert_eq!(hunk.deletions, vec!["    println!(\"Hello\");"]);
        assert_eq!(hunk.additions, vec!["    println!(\"Goodbye\");"]);
        assert_eq!(hunk.context_after, vec!["}"]);
        Ok(())
    }

    #[test]
    fn test_parse_multiple_hunks() -> Result<()> {
        let diff = "\
@@ @@
 fn one() {
-    1
+    2
 }
@@ @@
 fn two() {
-    3
+    4
 }";
        let fudiff = FuDiff::parse(diff)?;
        assert_eq!(fudiff.hunks.len(), 2);
        Ok(())
    }

    #[test]
    fn test_parse_with_file_headers() -> Result<()> {
        let diff = "\
--- a/src/main.rs
+++ b/src/main.rs
@@ @@
 fn main() {
-    1
+    2
 }";
        let fudiff = FuDiff::parse(diff)?;
        assert_eq!(fudiff.hunks.len(), 1);
        Ok(())
    }

    #[test]
    fn test_parse_error_no_hunks() {
        let diff = "just some\nrandom text";
        let result = FuDiff::parse(diff);
        println!("Result: {:?}", result); // Debug the actual error
        assert!(matches!(
            result,
            Err(Error::Parse(msg)) if msg == "No hunks found in diff"
        ));
    }

    #[test]
    fn test_parse_error_line_outside_hunk() {
        let diff = "line without hunk\n@@ @@\n context";
        assert!(matches!(
            FuDiff::parse(diff),
            Err(Error::Parse(msg)) if msg == "Line found outside of hunk"
        ));
    }

    #[test]
    fn test_parse_error_invalid_prefix() {
        let diff = "\
@@ @@
 context
# invalid";
        assert!(matches!(
            FuDiff::parse(diff),
            Err(Error::Parse(msg)) if msg.starts_with("Invalid line prefix")
        ));
    }
}
