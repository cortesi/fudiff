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
#[derive(Debug, Clone, PartialEq)]
pub struct Hunk {
    pub context_before: Vec<String>,
    pub deletions: Vec<String>,
    pub additions: Vec<String>,
    pub context_after: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strips leading whitespace from each line of the input string.
    /// Preserves relative indentation within the text while allowing test
    /// cases to be properly indented in the source.
    fn strip_leading_whitespace(text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return String::new();
        }

        // Find the minimum indentation level
        let min_indent = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        // Strip exactly that much whitespace from each line
        lines
            .iter()
            .map(|line| {
                if line.len() <= min_indent {
                    line.trim_start()
                } else {
                    &line[min_indent..]
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    struct TestCase {
        name: &'static str,
        input: &'static str,
        want_hunks: Option<Vec<Hunk>>,
        want_err: Option<&'static str>,
    }

    #[test]
    fn test_parse() {
        let tests = vec![
            TestCase {
                name: "basic hunk",
                input: "
                    @@ @@
                     fn main() {
                    -    println!(\"Hello\");
                    +    println!(\"Goodbye\");
                     }",
                want_hunks: Some(vec![Hunk {
                    context_before: vec!["fn main() {".to_string()],
                    deletions: vec!["    println!(\"Hello\");".to_string()],
                    additions: vec!["    println!(\"Goodbye\");".to_string()],
                    context_after: vec!["}".to_string()],
                }]),
                want_err: None,
            },
            TestCase {
                name: "multiple hunks",
                input: "
                    @@ @@
                     fn one() {
                    -    1
                    +    2
                     }
                    @@ @@
                     fn two() {
                    -    3
                    +    4
                     }",
                want_hunks: Some(vec![
                    Hunk {
                        context_before: vec!["fn one() {".to_string()],
                        deletions: vec!["    1".to_string()],
                        additions: vec!["    2".to_string()],
                        context_after: vec!["}".to_string()],
                    },
                    Hunk {
                        context_before: vec!["fn two() {".to_string()],
                        deletions: vec!["    3".to_string()],
                        additions: vec!["    4".to_string()],
                        context_after: vec!["}".to_string()],
                    },
                ]),
                want_err: None,
            },
            TestCase {
                name: "with file headers",
                input: "
                    --- a/src/main.rs
                    +++ b/src/main.rs
                    @@ @@
                     fn main() {
                    -    1
                    +    2
                     }",
                want_hunks: Some(vec![Hunk {
                    context_before: vec!["fn main() {".to_string()],
                    deletions: vec!["    1".to_string()],
                    additions: vec!["    2".to_string()],
                    context_after: vec!["}".to_string()],
                }]),
                want_err: None,
            },
            TestCase {
                name: "error - no hunks",
                input: "just some\nrandom text",
                want_hunks: None,
                want_err: Some("No hunks found in diff"),
            },
            TestCase {
                name: "error - line outside hunk",
                input: "line without hunk\n@@ @@\n context",
                want_hunks: None,
                want_err: Some("Line found outside of hunk"),
            },
            TestCase {
                name: "error - invalid prefix",
                input: "
                    @@ @@
                     context
                    # invalid",
                want_hunks: None,
                want_err: Some("Invalid line prefix"),
            },
        ];

        for test in tests {
            let result = FuDiff::parse(&strip_leading_whitespace(test.input));

            match (result, test.want_hunks, test.want_err) {
                (Ok(diff), Some(want_hunks), None) => {
                    assert_eq!(diff.hunks, want_hunks, "test case: {}", test.name);
                }
                (Err(Error::Parse(err)), None, Some(want_err)) => {
                    assert!(
                        err.contains(want_err),
                        "test case: {}\nwant error containing: {}\ngot: {}",
                        test.name,
                        want_err,
                        err
                    );
                }
                (result, want_hunks, want_err) => panic!(
                    "test case: {} - got result: {:?}, want_hunks: {:?}, want_err: {:?}",
                    test.name, result, want_hunks, want_err
                ),
            }
        }
    }
}
