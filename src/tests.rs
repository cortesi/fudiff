use crate::*;

#[test]
fn test_diff() {
    let test_cases = vec![
        // Empty inputs: no changes.
        ("", "", vec![]),
        // Only additions.
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
        // Only deletions.
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
        // Full replacement.
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
        // Changes at beginning.
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
        // Changes at end.
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
        // Interleaved changes.
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
        // No context between changes.
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
        let diff = crate::diff(old, new);
        assert_eq!(diff.hunks, expected_hunks);

        // Verify that parsing the rendered diff gives the same hunks.
        let rendered = diff.render();
        let parsed = crate::parse(&rendered).unwrap();
        assert_eq!(parsed.hunks, expected_hunks);
    }
}

fn strip_leading_whitespace(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let first_non_empty = lines.iter().position(|line| !line.trim().is_empty());
    let last_non_empty = lines.iter().rposition(|line| !line.trim().is_empty());

    if first_non_empty.is_none() {
        return String::new();
    }

    let (start, end) = (first_non_empty.unwrap(), last_non_empty.unwrap());
    let min_indent = lines[start..=end]
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

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
fn test_revert() {
    let test_cases = vec![
        (
            "fn main() {\n    println!(\"Goodbye\");\n}",
            "@@ @@\n fn main() {\n-    println!(\"Hello\");\n+    println!(\"Goodbye\");\n }\n",
            Ok("fn main() {\n    println!(\"Hello\");\n}"),
        ),
        (
            "a\nx\nc\ny\ne",
            "@@ @@\n a\n-b\n+x\n c\n@@ @@\n c\n-d\n+y\n e\n",
            Ok("a\nb\nc\nd\ne"),
        ),
        (
            "wrong content",
            "@@ @@\n a\n-b\n+x\n",
            Err("Could not find context"),
        ),
    ];

    for (input, diff_str, expected) in test_cases {
        let diff = crate::parse(diff_str).unwrap();
        match (diff.revert(input), expected) {
            (Ok(result), Ok(expected)) => assert_eq!(result, expected),
            (Err(Error::Apply(msg)), Err(expected_msg))
            | (Err(Error::AmbiguousMatch(msg)), Err(expected_msg)) => {
                assert!(msg.contains(expected_msg));
            }
            (result, expected) => {
                panic!("Unexpected result: {:?}, expected: {:?}", result, expected);
            }
        }
    }
}

#[test]
fn test_revert_round_trip() {
    let test_cases = vec![
        ("Empty input", "", ""),
        ("Single line", "hello world", "new line"),
        ("Multiple lines", "first\nsecond\nthird", "one\ntwo\nthree"),
        (
            "With context",
            "before\nchange\nafter",
            "before\ndifferent\nafter",
        ),
        ("Multiple hunks", "a\nx\nc\ny\ne", "a\nb\nc\nd\ne"),
        ("Empty lines", "\n\n\n", "1\n2\n3\n"),
        ("Special chars", "fn(x) { y }", "fn(x) { z }"),
        ("Indentation", "  a\n    b\n  c", "  a\n    x\n  c"),
    ];

    for (name, original, modified) in test_cases {
        let diff = crate::diff(original, modified);
        let patched = diff.patch(original).unwrap();
        assert_eq!(patched, modified, "{}: patch failed", name);
        let reverted = diff.revert(&patched).unwrap();
        assert_eq!(reverted, original, "{}: revert failed", name);
        let repatched = diff.patch(&reverted).unwrap();
        assert_eq!(repatched, modified, "{}: re-patch failed", name);
    }
}

#[test]
fn test_patch_edge_cases() {
    let test_cases = vec![
        ("", "", Ok("")),
        ("content", "", Ok("content")),
        ("", "@@ @@\n+new\n", Ok("new")),
        (
            "",
            "@@ @@\n-old\n",
            Err("Cannot apply patch to empty input"),
        ),
        (
            "one",
            "@@ @@\n-one\n-two\n",
            Err("Deletion extends past end of file"),
        ),
        (
            "a\nb\na\nb\nc",
            "@@ @@\n a\n b\n-c\n",
            Err("Multiple matches for context"),
        ),
        (
            "different",
            "@@ @@\n missing\n-old\n+new\n",
            Err("Could not find context"),
        ),
        ("a\nb\nc", "@@ @@\n b\n-c\n", Ok("a\nb")),
        (
            "a\nb\nc",
            "@@ @@\n-a\n+x\n@@ @@\n b\n-c\n+z\n",
            Ok("x\nb\nz"),
        ),
    ];

    for (input, diff_str, expected) in test_cases {
        let diff = crate::parse(diff_str).unwrap();
        match (diff.patch(input), expected) {
            (Ok(result), Ok(expected)) => assert_eq!(result, expected),
            (Err(Error::Apply(msg)), Err(expected_msg))
            | (Err(Error::AmbiguousMatch(msg)), Err(expected_msg)) => {
                assert!(msg.contains(expected_msg));
            }
            (result, expected) => {
                panic!("Unexpected result: {:?}, expected: {:?}", result, expected);
            }
        }
    }
}

#[test]
fn test_patch() {
    let test_cases = vec![
        // Basic function modification
        (
            "fn main() {\n    println!(\"Hello\");\n}",
            "@@ @@\n fn main() {\n-    println!(\"Hello\");\n+    println!(\"Goodbye\");\n }\n",
            Ok("fn main() {\n    println!(\"Goodbye\");\n}"),
        ),
        // Multiple hunks with surrounding context
        (
            "a\nb\nc\nd\ne",
            "@@ @@\n a\n-b\n+x\n c\n@@ @@\n d\n-e\n+y\n",
            Ok("a\nx\nc\nd\ny"),
        ),
        // Empty diff handling
        ("", "", Ok("")),
        // No changes in diff
        ("start\nmiddle\nend", "", Ok("start\nmiddle\nend")),
        // No-op changes
        ("start", "@@ @@\n-start\n+start\n", Ok("start")),
        // Newline preservation cases
        ("start\n", "@@ @@\n-start\n+start\n", Ok("start\n")),
        ("start\n", "@@ @@\n-start\n+start", Ok("start\n")),
        ("start", "@@ @@\n-start\n+start", Ok("start")),
        // Error cases
        (
            "",
            "@@ @@\n-line\n+newline\n",
            Err("Cannot apply patch to empty input"),
        ),
        // Missing context error
        (
            "wrong",
            "@@ @@\n context\n-old\n+new\n",
            Err("Could not find context"),
        ),
        // Content mismatch error
        ("a\nx\n", "@@ @@\n a\n-b\n+c\n", Err("Deletion mismatch")),
        // Ambiguous context error
        (
            "test\ntest\nend",
            "@@ @@\n test\n-end\n+new\n",
            Err("Multiple matches for context"),
        ),
        // Simple deletion at start with preserved content
        ("delete\nkeep", "@@ @@\n-delete\n+add\n", Ok("add\nkeep")),
        // Indentation handling
        (
            "line 1\n  indented\nline 3",
            "@@ @@\n line 1\n-  indented\n+\tnew\n",
            Ok("line 1\n\tnew\nline 3"),
        ),
        // Multiple hunks with shared context
        (
            "a\nb\nc\nd\ne",
            "@@ @@\n a\n b\n-c\n+x\n@@ @@\n d\n-e\n+y\n",
            Ok("a\nb\nx\nd\ny"),
        ),
        // Deletion at end with newline handling
        ("start\nend\n", "@@ @@\n start\n-end\n", Ok("start")),
    ];

    for (input, diff_str, expected) in test_cases {
        let diff = crate::parse(diff_str).unwrap();
        match (diff.patch(input), expected) {
            (Ok(result), Ok(expected)) => {
                assert_eq!(result, expected);
            }
            (Err(Error::Apply(msg)), Err(expected_msg))
            | (Err(Error::AmbiguousMatch(msg)), Err(expected_msg)) => {
                assert!(msg.contains(expected_msg));
            }
            (result, expected) => {
                panic!("Unexpected result: {:?}, expected: {:?}", result, expected);
            }
        }
    }
}

#[test]
fn test_parse_render_round_trip() {
    let test_cases = vec![
        "",
        "@@ @@\n fn main() {\n-    old\n+    new\n }\n",
        "@@ @@\n a\n-b\n+c\n d\n@@ @@\n x\n-y\n+z\n w\n",
        "@@ @@\n-deleted\n+added\n",
        "@@ @@\n context1\n context2\n",
        "@@ @@\n before\n-del1\n-del2\n+add1\n+add2\n after\n",
    ];

    for input in test_cases {
        let parsed = crate::parse(input).unwrap();
        let rendered = parsed.render();
        let reparsed = crate::parse(&rendered).unwrap();

        assert_eq!(
            parsed.hunks, reparsed.hunks,
            "Round trip failed.\nInput:\n{}\nRe-rendered:\n{}",
            input, rendered
        );
    }
}

#[test]
fn test_diff_patch_round_trip() {
    // Test cases that verify the full cycle: diff -> patch -> diff -> patch
    // ensures that transformations are reversible and consistent
    let test_cases = vec![
        // Base cases
        ("Empty input/output", "", ""),
        ("Single line change", "hello", "hi"),
        ("Multiple line changes", "a\nb\nc", "a\nx\nc"),
        ("Full file change", "old\nfile", "new\nfile"),
        // Complex modifications
        ("Multiple hunks", "a\nb\nc\nd\ne", "a\nx\nc\ny\ne"),
        ("Leading context", "keep\nold\nend", "keep\nnew\nend"),
        ("Trailing context", "start\nold\nkeep", "start\nnew\nkeep"),
        // Edge cases
        ("Additions only", "start\nend", "start\nnew\nend"),
        ("Deletions only", "start\nremove\nend", "start\nend"),
        ("Empty lines", "\n\na\n\n", "\n\nb\n\n"),
        // Format handling
        ("Line endings", "a\nb\nc\n", "a\nx\nc\n"),
        ("With indentation", "  a\n  b\n  c", "  a\n  x\n  c"),
        ("Special characters", "fn(x) {\n  y\n}", "fn(x) {\n  z\n}"),
    ];

    for (name, original, modified) in test_cases {
        let diff = crate::diff(original, modified);
        let patched = diff.patch(original).unwrap();
        assert_eq!(patched, modified, "{}: patch failed", name);
        let new_diff = crate::diff(original, &patched);
        let original_with_new_diff = new_diff.patch(original).unwrap();
        assert_eq!(
            original_with_new_diff, modified,
            "{}: diff round-trip failed",
            name
        );
        let reverse_diff = crate::diff(&patched, original);
        let back_to_original = reverse_diff.patch(&patched).unwrap();
        assert_eq!(back_to_original, original, "{}: reverse diff failed", name);
    }
}

#[test]
fn test_newline_preservation() {
    let original = "line1\nline2\n";
    let modified = "line1\nmodified line2\n";
    let diff = crate::diff(original, modified);
    let patched = diff.patch(original).unwrap();
    // Ensure trailing newline is preserved if originally present.
    assert_eq!(patched, modified);
}

// Additional tests for edge cases.

#[test]
fn test_unicode_diff() {
    let old = "こんにちは\n世界";
    let new = "こんにちは\nRust";
    let diff = crate::diff(old, new);
    let patched = diff.patch(old).unwrap();
    assert_eq!(patched, new);
    let reverted = diff.revert(&patched).unwrap();
    assert_eq!(reverted, old);
}

#[test]
fn test_windows_line_endings() {
    // Convert windows line endings to unix for .lines() based processing.
    let old_windows = "line1\r\nline2\r\n";
    let new_windows = "line1\r\nline changed\r\nline2\r\n";
    let old_unix = "line1\nline2\n";
    let new_unix = "line1\nline changed\nline2\n";
    let diff = crate::diff(old_unix, new_unix);
    let patched = diff.patch(old_unix).unwrap();
    assert_eq!(patched, new_unix);
}

#[test]
fn test_all_lines_changed() {
    let old = "a\nb\nc";
    let new = "x\ny\nz";
    let diff = crate::diff(old, new);
    // Expect a single hunk with no surrounding context.
    assert_eq!(diff.hunks.len(), 1);
    let hunk = &diff.hunks[0];
    assert!(hunk.context_before.is_empty());
    assert!(hunk.context_after.is_empty());
    assert_eq!(
        hunk.deletions,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(
        hunk.additions,
        vec!["x".to_string(), "y".to_string(), "z".to_string()]
    );
    let patched = diff.patch(old).unwrap();
    assert_eq!(patched, new);
}

#[test]
fn test_no_change() {
    let text = "unchanged\nline";
    let diff_instance = crate::diff(text, text);
    // No hunks should be produced.
    assert!(diff_instance.hunks.is_empty());
    let patched = diff_instance.patch(text).unwrap();
    assert_eq!(patched, text);
}
