//! Implementation of the Fuzzy Unified Diff Format.
//! This module provides functions to compute, render, parse, apply, and revert fuzzy diffs.

#[cfg(test)]
mod tests;

/// Error type for FuDiff operations.
#[derive(Debug)]
pub enum Error {
    /// Failed to parse the diff format.
    Parse(String),
    /// Failed to apply the patch.
    Apply(String),
    /// Multiple possible matches found for context.
    AmbiguousMatch(String),
}

/// A type alias for diff operation results.
pub type Result<T> = std::result::Result<T, Error>;

/// Computes the fuzzy diff between the given 'old' and 'new' strings.
/// Returns a FuDiff representing the hunks of changes.
pub fn diff(old: &str, new: &str) -> FuDiff {
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
        let lookahead = 3;
        let mut next_match = None;

        // Look for nearest matching lines within the lookahead window.
        for offset in 0..=lookahead {
            for di in 0..=offset {
                for dj in 0..=offset {
                    if i + di < old_lines.len()
                        && j + dj < new_lines.len()
                        && (di > 0 || dj > 0)
                        && old_lines[i + di] == new_lines[j + dj]
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
                // Accumulate differing lines.
                current_hunk
                    .deletions
                    .extend(old_lines[i..i + di].iter().map(|s| s.to_string()));
                current_hunk
                    .additions
                    .extend(new_lines[j..j + dj].iter().map(|s| s.to_string()));
                i += di;
                j += dj;

                // Collect matching context lines after the change.
                let mut matches = 0;
                while i + matches < old_lines.len()
                    && j + matches < new_lines.len()
                    && old_lines[i + matches] == new_lines[j + matches]
                    && matches < lookahead
                {
                    current_hunk
                        .context_after
                        .push(old_lines[i + matches].to_string());
                    matches += 1;
                }
                i += matches;
                j += matches;

                // Finalize the current hunk if it contains changes.
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
                // No further match found: add all remaining lines.
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

    // Add the final hunk if any changes exist.
    if !current_hunk.deletions.is_empty() || !current_hunk.additions.is_empty() {
        hunks.push(current_hunk);
    }

    FuDiff { hunks }
}

/// Parses a unified diff format string into a FuDiff.
/// Returns an error if no valid hunks are found or if parsing fails.
pub fn parse(input: &str) -> Result<FuDiff> {
    let mut hunks = Vec::new();
    let mut current_hunk = None;

    // Empty input signifies a diff with no changes.
    if input.trim().is_empty() {
        return Ok(FuDiff { hunks: vec![] });
    }

    // Non-empty input must contain hunk markers.
    if !input.contains("@@") {
        return Err(Error::Parse("No hunks found in diff".to_string()));
    }

    for line in input.lines() {
        if line.starts_with("@@") && line[2..].contains("@@") {
            // Finalize the previous hunk and start a new one.
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

        // Skip headers and irrelevant lines.
        if line.is_empty() || line.starts_with("---") || line.starts_with("+++") {
            continue;
        }

        // Ensure the line is within a hunk.
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

    // Append the final hunk if present.
    if let Some(hunk) = current_hunk.take() {
        hunks.push(hunk);
    }

    Ok(FuDiff { hunks })
}

/// Represents a complete fuzzy diff consisting of multiple hunks.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FuDiff {
    pub hunks: Vec<Hunk>,
}

impl std::fmt::Display for FuDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl FuDiff {
    /// Reverts the changes represented by this diff from the given input.
    /// This swaps additions with deletions and applies the patch.
    pub fn revert(&self, input: &str) -> Result<String> {
        let reverted = FuDiff {
            hunks: self
                .hunks
                .iter()
                .map(|h| Hunk {
                    context_before: h.context_before.clone(),
                    deletions: h.additions.clone(),
                    additions: h.deletions.clone(),
                    context_after: h.context_after.clone(),
                })
                .collect(),
        };

        reverted.patch(input)
    }

    /// Applies this diff to the provided input text, returning the patched result.
    /// Returns an error if the patch cannot be applied cleanly.
    pub fn patch(&self, input: &str) -> Result<String> {
        if self.hunks.is_empty() {
            return Ok(input.to_string());
        }

        let lines: Vec<&str> = input.lines().collect();
        // Allow empty input if hunks have only additions.
        if lines.is_empty() && self.hunks.iter().any(|h| !h.deletions.is_empty()) {
            return Err(Error::Apply(
                "Cannot apply patch to empty input".to_string(),
            ));
        }

        let mut result = Vec::new();
        let mut pos = 0;

        for hunk in &self.hunks {
            // Locate the position in input where this hunk should be applied using context.
            let hunk_pos = if hunk.context_before.is_empty() {
                pos
            } else {
                let mut found_pos = None;
                'outer: for i in pos..=lines.len().saturating_sub(hunk.context_before.len()) {
                    for (j, line) in hunk.context_before.iter().enumerate() {
                        if i + j >= lines.len() || lines[i + j] != line {
                            continue 'outer;
                        }
                    }
                    if found_pos.is_some() {
                        return Err(Error::AmbiguousMatch(format!(
                            "Multiple matches for context: {:?}",
                            hunk.context_before
                        )));
                    }
                    found_pos = Some(i);
                }
                found_pos.ok_or_else(|| {
                    Error::Apply(format!("Could not find context: {:?}", hunk.context_before))
                })?
            };

            // Verify that the deletion lines match the corresponding lines in the input.
            let deletion_start = hunk_pos + hunk.context_before.len();
            if !hunk.deletions.is_empty() {
                if deletion_start + hunk.deletions.len() > lines.len() {
                    return Err(Error::Apply(
                        "Deletion extends past end of file".to_string(),
                    ));
                }
                for (i, deletion) in hunk.deletions.iter().enumerate() {
                    if lines[deletion_start + i] != deletion {
                        return Err(Error::Apply(format!(
                            "Deletion mismatch at line {} - expected '{}', found '{}'",
                            deletion_start + i + 1,
                            deletion,
                            lines[deletion_start + i]
                        )));
                    }
                }
            }

            // Append unchanged lines preceding the hunk.
            if pos < hunk_pos {
                result.extend(lines[pos..hunk_pos].iter().map(|s| s.to_string()));
            }

            // Append the preserved context from the original input and new additions.
            result.extend(
                hunk.context_before
                    .iter()
                    .enumerate()
                    .map(|(i, _)| lines[hunk_pos + i].to_string()),
            );
            result.extend(hunk.additions.iter().cloned());

            // Advance the position past the deletion section.
            pos = hunk_pos + hunk.context_before.len() + hunk.deletions.len();
        }

        // Append any remaining lines from the input.
        if pos < lines.len() {
            result.extend(lines[pos..].iter().map(|s| s.to_string()));
        }

        // Build the output string and preserve trailing newline if appropriate.
        let mut output = result.join("\n");
        if !result.is_empty() {
            let has_input_newline = input.ends_with('\n');
            let mut has_output_newline = false;

            if let Some(last_hunk) = self.hunks.last() {
                if !last_hunk.context_after.is_empty() || !last_hunk.additions.is_empty() {
                    has_output_newline = has_input_newline;
                }
            } else {
                has_output_newline = has_input_newline;
            }

            if has_output_newline {
                output.push('\n');
            }
        }
        Ok(output)
    }

    /// Renders the diff into a unified diff format string.
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
}

/// Represents a single hunk of changes within a diff.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hunk {
    pub context_before: Vec<String>,
    pub deletions: Vec<String>,
    pub additions: Vec<String>,
    pub context_after: Vec<String>,
}
