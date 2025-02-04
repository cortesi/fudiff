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
    /// Creates a diff between two strings.
    pub fn diff(old: &str, new: &str) -> Self {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        let mut hunks = Vec::new();
        let mut current_hunk = Hunk {
            context_before: Vec::new(),
            deletions: Vec::new(),
            additions: Vec::new(),
            context_after: Vec::new(),
        };

        let mut i = 0;
        let mut j = 0;

        while i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            current_hunk.context_before.push(old_lines[i].to_string());
            i += 1;
            j += 1;
        }

        while i < old_lines.len() || j < new_lines.len() {
            let look_ahead = 3;
            let mut next_match = None;

            // Look for nearest match within look_ahead window
            for offset in 0..=look_ahead {
                let _max_i = usize::min(i + offset, old_lines.len());
                let _max_j = usize::min(j + offset, new_lines.len());

                for di in 0..=offset {
                    for dj in 0..=offset {
                        if i + di < old_lines.len()
                            && j + dj < new_lines.len()
                            && old_lines[i + di] == new_lines[j + dj]
                            && (di > 0 || dj > 0)
                        {
                            next_match = Some((di, dj));
                            break;
                        }
                    }
                    if next_match.is_some() {
                        break;
                    }
                }
                if next_match.is_some() {
                    break;
                }
            }

            match next_match {
                Some((di, dj)) => {
                    // Add differing lines as changes
                    current_hunk
                        .deletions
                        .extend(old_lines[i..i + di].iter().map(|s| s.to_string()));
                    current_hunk
                        .additions
                        .extend(new_lines[j..j + dj].iter().map(|s| s.to_string()));
                    i += di;
                    j += dj;

                    // Add matching context lines
                    let mut matches = 0;
                    while i + matches < old_lines.len()
                        && j + matches < new_lines.len()
                        && old_lines[i + matches] == new_lines[j + matches]
                        && matches < look_ahead
                    {
                        current_hunk
                            .context_after
                            .push(old_lines[i + matches].to_string());
                        matches += 1;
                    }
                    i += matches;
                    j += matches;

                    // Finalize current hunk and start new one if needed
                    if !current_hunk.deletions.is_empty() || !current_hunk.additions.is_empty() {
                        let mut new_hunk = Hunk {
                            context_before: Vec::new(),
                            deletions: Vec::new(),
                            additions: Vec::new(),
                            context_after: Vec::new(),
                        };
                        std::mem::swap(
                            &mut new_hunk.context_before,
                            &mut current_hunk.context_after,
                        );
                        hunks.push(current_hunk);
                        current_hunk = new_hunk;
                    }
                }
                None => {
                    // Add all remaining lines
                    current_hunk
                        .deletions
                        .extend(old_lines[i..].iter().map(|s| s.to_string()));
                    current_hunk
                        .additions
                        .extend(new_lines[j..].iter().map(|s| s.to_string()));
                    break;
                }
            }
        }

        // Add final hunk if it contains changes
        if !current_hunk.deletions.is_empty() || !current_hunk.additions.is_empty() {
            hunks.push(current_hunk);
        }

        FuDiff { hunks }
    }

    /// Renders the diff back to the unified diff format.
    pub fn render(&self) -> String {
        let mut output = String::new();

        for (i, hunk) in self.hunks.iter().enumerate() {
            output.push_str("@@ @@\n");

            for line in &hunk.context_before {
                output.push(' ');
                output.push_str(line);
                output.push('\n');
            }

            for line in &hunk.deletions {
                output.push('-');
                output.push_str(line);
                output.push('\n');
            }

            for line in &hunk.additions {
                output.push('+');
                output.push_str(line);
                output.push('\n');
            }

            for (j, line) in hunk.context_after.iter().enumerate() {
                output.push(' ');
                output.push_str(line);
                if i < self.hunks.len() - 1 || j < hunk.context_after.len() - 1 {
                    output.push('\n');
                }
            }
        }

        output
    }

    /// Parse a fuzzy diff from a string.
    pub fn parse(input: &str) -> Result<Self> {
        let mut hunks = Vec::new();
        let mut current_hunk = None;

        // Empty input is valid for a diff with no changes
        if input.trim().is_empty() {
            return Ok(FuDiff { hunks: vec![] });
        }

        // Non-empty input must contain hunk markers
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

    #[test]
    fn test_diff() {
        let test_cases = vec![
            // Empty inputs
            ("", "", vec![]),
            (
                "",
                "a\nb",
                vec![Hunk {
                    context_before: vec![],
                    deletions: vec![],
                    additions: vec!["a".to_string(), "b".to_string()],
                    context_after: vec![],
                }],
            ),
            (
                "x\ny",
                "",
                vec![Hunk {
                    context_before: vec![],
                    deletions: vec!["x".to_string(), "y".to_string()],
                    additions: vec![],
                    context_after: vec![],
                }],
            ),
            // Full replacement
            (
                "old",
                "new",
                vec![Hunk {
                    context_before: vec![],
                    deletions: vec!["old".to_string()],
                    additions: vec!["new".to_string()],
                    context_after: vec![],
                }],
            ),
            // Changes at beginning
            (
                "a\nb\nc",
                "x\ny\nc",
                vec![Hunk {
                    context_before: vec![],
                    deletions: vec!["a".to_string(), "b".to_string()],
                    additions: vec!["x".to_string(), "y".to_string()],
                    context_after: vec![],
                }],
            ),
            // Changes at end
            (
                "a\nb\nc",
                "a\nx\ny",
                vec![Hunk {
                    context_before: vec!["a".to_string()],
                    deletions: vec!["b".to_string(), "c".to_string()],
                    additions: vec!["x".to_string(), "y".to_string()],
                    context_after: vec![],
                }],
            ),
            // Interleaved changes
            (
                "a\nb\nc\nd\ne",
                "a\nx\nc\ny\ne",
                vec![
                    Hunk {
                        context_before: vec!["a".to_string()],
                        deletions: vec!["b".to_string()],
                        additions: vec!["x".to_string()],
                        context_after: vec![],
                    },
                    Hunk {
                        context_before: vec!["c".to_string()],
                        deletions: vec!["d".to_string()],
                        additions: vec!["y".to_string()],
                        context_after: vec![],
                    },
                ],
            ),
            // No context between changes
            (
                "a\nb\nc",
                "x\ny\nz",
                vec![Hunk {
                    context_before: vec![],
                    deletions: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    additions: vec!["x".to_string(), "y".to_string(), "z".to_string()],
                    context_after: vec![],
                }],
            ),
        ];

        for (old, new, expected_hunks) in test_cases {
            let diff = FuDiff::diff(old, new);
            assert_eq!(diff.hunks, expected_hunks);

            // Verify that parsing the rendered diff gives same hunks
            let rendered = diff.render();
            let parsed = FuDiff::parse(&rendered).unwrap();
            assert_eq!(parsed.hunks, expected_hunks);
        }
    }

    /// Strips leading whitespace from each line of the input string.
    /// Preserves relative indentation within the text while allowing test
    /// cases to be properly indented in the source.
    fn strip_leading_whitespace(text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return String::new();
        }

        // Find non-empty line indices
        let first_non_empty = lines.iter().position(|line| !line.trim().is_empty());
        let last_non_empty = lines.iter().rposition(|line| !line.trim().is_empty());

        if first_non_empty.is_none() {
            return String::new();
        }

        let (start, end) = (first_non_empty.unwrap(), last_non_empty.unwrap());

        // Find the minimum indentation level among non-empty lines
        let min_indent = lines[start..=end]
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        // Strip exactly that much whitespace from each line
        lines[start..=end]
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

        let expected = "
            @@ @@
             fn main() {
            -    println!(\"Hello\");
            +    println!(\"Goodbye\");
             }
        ";

        assert_eq!(diff.render(), strip_leading_whitespace(expected));
    }

    #[test]
    fn test_round_trip() {
        let test_cases = vec![
            // Empty diff
            "",
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
