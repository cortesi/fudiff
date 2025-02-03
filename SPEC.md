# Fuzzy Unified Diff Format 

## 1. Introduction

This specification describes an fuzzy unified diff format intended for systems
where AI models output patches to modify text files. It is based on the classic
unified diff format but has been modified to address common challenges:

- **Avoiding explicit line numbers:** AI models often have difficulty producing
  correct line indices.
- **Context-based matching:** The diff relies solely on unchanged context lines
  to locate the correct patch position.
- **Clear demarcation of changes:** Simple markers indicate removed and added
  lines.

This format is intended for text-based files (such as source code or
configuration files) and is designed for integration into patching tools
written in Rust or similar languages.

## 2. Scope

This format applies to text files only; it is not intended for binary diffs.
The specification covers:

- File headers to identify the target file.
- Hunk headers that demarcate the start of a change block.
- Hunk bodies that contain context lines, deletions, and additions.

## 3. Overall Structure

An AIFUD document consists of three main parts:

1. **File Headers:** Identify the file to be patched.
2. **Hunk Headers:** Mark the start of each change block.
3. **Hunk Bodies:** Contain the actual context lines and changes.

### 3.1 File Headers

The file header appears at the beginning of the diff. Unlike standard diffs
that include timestamps, this version emphasizes minimal metadata. For example:

```
--- filename: path/to/file.ext
+++ filename: path/to/file.ext
```

**Rules:**
- The header lines must start with `---` for the original file and `+++` for the updated file.
- The filename is specified after the label (`filename:`) to ensure consistency.
- No timestamps or additional metadata is included.

### 3.2 Hunk Headers

Each hunk header is written in a simplified format that does not include line
numbers or any extra metadata. The header consists simply of a pair of `@@`
symbols:

```
@@ @@
```

**Rules:**

- The header must appear on a single line by itself.
- No line numbers or context hashes are provided. The patching tool relies
  solely on the context lines that follow to locate the correct patch position.

### 3.3 Hunk Bodies

The hunk body is a sequence of lines that represent:
- **Context lines:** Unchanged lines (prefixed with a single space `" "`).
- **Deletions:** Lines removed from the original file (prefixed with a minus `"-"`).
- **Additions:** Lines added in the new file (prefixed with a plus `"+"`).

**General Format:**

```diff
@@ @@
<unchanged context lines>
-<line removed>
+<line added>
<unchanged context lines>
```

**Rules:**

- Every line in the hunk must begin with one of the three prefixes: a space, a minus, or a plus.
- Context lines (beginning with a space) serve to locate the patch insertion point via fuzzy matching.
- A deletion line indicates that the specified text must exist in the target file and will be removed.
- An addition line indicates that the given text will be inserted at that point.

## 4. Detailed Syntax and Semantics

### 4.1 File Header Details

- **Original file header:**  
  Must begin with `--- filename: ` immediately followed by the file path.
- **Updated file header:**  
  Must begin with `+++ filename: ` immediately followed by the file path.
- The file headers are mandatory and must appear as the first non-comment lines in the diff.

### 4.2 Hunk Header Details

- The hunk header is always on a line by itself and appears as:
  
  ```
  @@ @@
  ```
  
- No additional identifiers, line numbers, or hashes are included.
- The patching tool uses the subsequent context lines for locating the correct section in the target file.

### 4.3 Hunk Body Details

Each hunk body should ideally include a minimum of two context lines before and
after a change, although shorter context may be acceptable at the file
boundaries.

**Line Prefixes:**

- **Context line:**  
  Begins with a single space.  
  *Example:*
  ```
   fn main() {
  ```
- **Deletion line:**  
  Begins with a minus sign.  
  *Example:*
  ```
  -    println!("Hello, world!");
  ```
- **Addition line:**  
  Begins with a plus sign.  
  *Example:*
  ```
  +    println!("Goodbye, world!");
  ```

## 5. AI-Focused Considerations

### 5.1 Omission of Line Numbers

Traditional unified diffs include line numbers (e.g., `@@ -1,4 +1,4 @@`), but
these are omitted in this format to avoid AI errors. The patching tool locates
the patch area using the surrounding context.

### 5.2 Fuzzy Matching via Context

Since exact line numbers are not provided:
- The patching engine must use fuzzy matching algorithms to locate the block of
  context lines in the target file.
- The engine should allow for minor differences such as whitespace changes.
- A minimum of 2–3 context lines before and after the change is recommended to
  reduce ambiguity.
- If multiple matches are found, the engine should either prompt the user for
  clarification or choose the first unique match.

### 5.3 Handling Special Characters

- Lines with special characters (e.g., `@`, `#`, `<`, `>`) are treated as
  literal text.
- No additional escaping is required beyond the first character, which serves
  as the marker.
- If a file’s content includes the marker characters at the start of a line, a
  pre-processing step (or an escaping mechanism) must be applied before
  generating the diff.

### 5.4 Multi-Hunk and Overlapping Changes

- Multiple hunks may appear sequentially in one diff file. They should be
  processed in order.
- If hunks are adjacent or overlap, the patching engine should merge them if
  possible. Otherwise, it should report a conflict and request manual
  resolution.
- In cases where context lines are sparse (e.g., changes near the beginning or
  end of a file), the engine should allow “partial” context matching.

## 6. Examples

### 6.1 Simple Single-Line Replacement

Suppose you have a file `example.rs` with the following content:

```rust
fn main() {
    println!("Hello, world!");
}
```

To update the greeting, the diff would be:

```diff
--- filename: example.rs
+++ filename: example.rs
@@ @@
  fn main() {
-    println!("Hello, world!");
+    println!("Goodbye, world!");
  }
```

### 6.2 Multi-Line Change with Context

Consider a function where multiple lines need modification:

**Original file (`compute.rs`):**

```rust
fn compute(x: i32) -> i32 {
    let y = x * 2;
    println!("The value is {}", y);
    y
}
```

**Desired change:** Replace multiplication with addition and update the print message.

**Diff:**

```diff
--- filename: compute.rs
+++ filename: compute.rs
@@ @@
  fn compute(x: i32) -> i32 {
-    let y = x * 2;
+    let y = x + 2;
-    println!("The value is {}", y);
+    println!("The computed value is {}", y);
     y
  }
```

### 6.3 Change at File Boundary

For a change at the very beginning of a file (e.g., adding a header comment):

**Original file (`main.rs`):**

```rust
fn main() {
    println!("Hello!");
}
```

**Diff:**

```diff
--- filename: main.rs
+++ filename: main.rs
@@ @@
-# (no header)
+// This file was updated on 2025-02-04
  fn main() {
      println!("Hello!");
  }
```

*Note:* Here the deletion indicates an empty placeholder for a missing header.
The patch engine should interpret this as an insertion at the beginning.

## 7. Corner Cases and Error Handling

### 7.1 Ambiguous Context

- **Scenario:** The same block of context appears in multiple locations.
- **Specification:** The patch engine should either:
  - Use a fallback strategy (e.g., apply the patch to the first unique match)
    or
  - Fail gracefully, prompting the user to resolve the ambiguity.

### 7.2 Incomplete Hunk Bodies

- **Scenario:** A hunk is missing either a deletion or addition line.
- **Specification:**  
  - A hunk with only context lines is treated as a no-op (useful for
    verification). A hunk with only deletions or only additions (outside of
    file-boundary insertions) should be flagged as an error.

### 7.3 Handling Trailing Newlines

- **Scenario:** Differences in trailing newlines could cause mismatches.
- **Specification:**  
  - The patch engine must normalize trailing newlines before matching context.
  - If a patch adds or removes a trailing newline, this should be explicitly
    represented (e.g., an empty line with the appropriate prefix).

### 7.4 Lines Starting with Marker Characters

- **Scenario:** A file line begins with a space, `-`, or `+` naturally.
- **Specification:**  
  - The patch engine should reserve the first character in each hunk line for
    its marker.
  - If necessary, implement a pre-processing step to escape such lines before
    diff generation, and unescape them when applying the patch.

### 7.5 Overlapping and Adjacent Hunks

- **Scenario:** Two hunks are adjacent or have overlapping context.
- **Specification:**  
  - The patch engine should attempt to merge adjacent hunks if their contexts
    overlap.
  - If merging is not possible, the engine must report a conflict and require
    manual resolution.

## 8. Implementation Recommendations (in Rust)

When implementing this specification in Rust, consider the following
suggestions:

- **Parsing:**  
  - Use a line-by-line parser that distinguishes file header lines and hunk
    sections.
  - A state machine approach is recommended:
    - **State 1:** Read file headers.
    - **State 2:** On encountering `@@ @@`, begin a new hunk.
    - **State 3:** Process subsequent hunk body lines until the next hunk
      header or end-of-file.
  
- **Fuzzy Matching:**  
  - Implement a fuzzy matching algorithm to locate the context block in the
    target file. Libraries such as
    [`fuzzy-matcher`](https://crates.io/crates/fuzzy-matcher) might be useful.
  - Normalize whitespace and trailing newlines to reduce false mismatches.

- **Error Reporting:**  
  - Provide clear error messages for ambiguous contexts, incomplete hunks, or
    parsing failures to assist debugging and user feedback.

- **Testing:**  
  - Write comprehensive tests for all corner cases described, ensuring that the
    patch application logic handles various diff scenarios robustly.
