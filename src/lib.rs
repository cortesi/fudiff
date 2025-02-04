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
#[derive(Debug)]
pub struct FuDiff {
    pub hunks: Vec<Hunk>,
}

impl std::fmt::Display for FuDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl FuDiff {
    /// Renders the diff back to the unified diff format.
    pub fn render(&self) -> String {
        let mut output = String::new();

        for hunk in &self.hunks {
            output.push_str("@@ @@\n");

            for line in &hunk.context_before {
                output.push_str(" ");
                output.push_str(line);
                output.push('\n');
            }

            for line in &hunk.deletions {
                output.push_str("-");
                output.push_str(line);
                output.push('\n');
            }

            for line in &hunk.additions {
                output.push_str("+");
                output.push_str(line);
                output.push('\n');
            }

            for line in &hunk.context_after {
                output.push_str(" ");
                output.push_str(line);
                output.push('\n');
            }
        }

        output
    }

    /// Parse a fuzzy diff from a string.
    pub fn parse(input: &str) -> Result<Self> {
        let mut hunks = Vec::new();
        let mut current_hunk = None;

        // Fast-fail if no hunk markers present
        if !input.contains("@@") {
            return Err(Error::Parse("No hunks found in diff".to_string()));
        }

        for line in input.lines() {
            if line.starts_with("@@") && line[2..].contains("@@") {
                // Finalize current hunk and start new one, ignoring text between @@ markers
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

            // Skip irrelevant lines
            if line.is_empty() || line.starts_with("---") || line.starts_with("+++") {
                continue;
            }

            // Require lines to be in a hunk context
            let hunk = current_hunk
                .as_mut()
                .ok_or_else(|| Error::Parse("Line found outside of hunk".to_string()))?;

            let (marker, content) = line.split_at(1);
            match marker {
                " " => {
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

        // Capture final hunk if present
        if let Some(hunk) = current_hunk.take() {
            hunks.push(hunk);
        }

        if hunks.is_empty() {
            Err(Error::Parse("No hunks found in diff".to_string()))
        } else {
            Ok(FuDiff { hunks })
        }
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
    fn test_render() {
        let diff = FuDiff {
            hunks: vec![Hunk {
                context_before: vec!["fn main() {".to_string()],
                deletions: vec!["    println!(\"Hello\");".to_string()],
                additions: vec!["    println!(\"Goodbye\");".to_string()],
                context_after: vec!["}".to_string()],
            }],
        };

        let expected = "\
@@ @@
 fn main() {
-    println!(\"Hello\");
+    println!(\"Goodbye\");
 }
";

        assert_eq!(diff.render(), expected);
    }

    #[test]
    fn test_round_trip() {
        let test_cases = vec![
            // Basic round trip
            "@@ @@\n fn main() {\n-    old\n+    new\n }\n",
            // Multiple hunks
            "@@ @@\n a\n-b\n+c\n d\n@@ @@\n x\n-y\n+z\n w\n",
            // Empty context sections
            "@@ @@\n-deleted\n+added\n",
            // Just context
            "@@ @@\n context1\n context2\n",
            // Multiple deletions and additions
            "@@ @@\n before\n-del1\n-del2\n+add1\n+add2\n after\n",
        ];

        for input in test_cases {
            let parsed = FuDiff::parse(input).unwrap();
            let rendered = parsed.render();
            let reparsed = FuDiff::parse(&rendered).unwrap();

            assert_eq!(
                parsed.hunks, reparsed.hunks,
                "Round trip failed.\nInput:\n{}\nRe-rendered:\n{}",
                input, rendered
            );
        }
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
            TestCase {
                name: "hunk headers are ignored",
                input: "
                    @@ -1,3 +1,3 @@ some text here
                     fn test() {
                    -    old();
                    +    new();
                     }
                    @@ -10,2 +10,2 @@ more header text
                     other() {
                    -    a();
                    +    b();
                     }",
                want_hunks: Some(vec![
                    Hunk {
                        context_before: vec!["fn test() {".to_string()],
                        deletions: vec!["    old();".to_string()],
                        additions: vec!["    new();".to_string()],
                        context_after: vec!["}".to_string()],
                    },
                    Hunk {
                        context_before: vec!["other() {".to_string()],
                        deletions: vec!["    a();".to_string()],
                        additions: vec!["    b();".to_string()],
                        context_after: vec!["}".to_string()],
                    },
                ]),
                want_err: None,
            },
            TestCase {
                name: "multi-line changes",
                input: "
                    @@ @@
                     fn test() {
                     let x = 10;
                    -    if true {
                    -        println!(\"a\");
                    -        println!(\"b\");
                    -    }
                    +    match x {
                    +        10 => println!(\"ten\"),
                    +        _ => println!(\"other\"),
                    +    }
                     let y = 20;
                     return y;
                     }",
                want_hunks: Some(vec![Hunk {
                    context_before: vec!["fn test() {".to_string(), "let x = 10;".to_string()],
                    deletions: vec![
                        "    if true {".to_string(),
                        "        println!(\"a\");".to_string(),
                        "        println!(\"b\");".to_string(),
                        "    }".to_string(),
                    ],
                    additions: vec![
                        "    match x {".to_string(),
                        "        10 => println!(\"ten\"),".to_string(),
                        "        _ => println!(\"other\"),".to_string(),
                        "    }".to_string(),
                    ],
                    context_after: vec![
                        "let y = 20;".to_string(),
                        "return y;".to_string(),
                        "}".to_string(),
                    ],
                }]),
                want_err: None,
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
