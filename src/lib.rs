//! Implementation of the Fuzzy Unified Diff Format.
//! This module provides functions to compute, render, parse, apply, and revert fuzzy diffs.

#[cfg(test)]
mod tests;

/// Error type for FuDiff operations.
#[derive(Debug)]
pub enum Error {
    /// Failed to parse the diff format.
    Parse { user: String, details: String },
    /// Failed to apply the patch.
    Apply { user: String, details: String },
    /// Multiple possible matches found for context.
    AmbiguousMatch { user: String, details: String },
}

impl Error {
    /// Retrieves the detailed error message.
    pub fn details(&self) -> &str {
        match self {
            Error::Parse { details, .. } => details,
            Error::Apply { details, .. } => details,
            Error::AmbiguousMatch { details, .. } => details,
        }
    }
}

/// A type alias for diff operation results.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents a single hunk of changes within a diff.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hunk {
    pub context_before: Vec<String>,
    pub deletions: Vec<String>,
    pub additions: Vec<String>,
    pub context_after: Vec<String>,
}

/// Represents a complete fuzzy diff consisting of multiple hunks.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
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
        if lines.is_empty() && self.hunks.iter().any(|h| !h.deletions.is_empty()) {
            return Err(Error::Apply {
                user: "Failed to apply patch".to_string(),
                details: "Cannot apply patch to empty input".to_string(),
            });
        }

        let mut result = Vec::new();
        let mut pos = 0;

        for hunk in &self.hunks {
            let hunk_pos = if hunk.context_before.is_empty() {
                pos
            } else {
                let mut candidate = None;
                for i in pos..=lines.len().saturating_sub(hunk.context_before.len()) {
                    if hunk
                        .context_before
                        .iter()
                        .enumerate()
                        .all(|(j, ctx)| lines[i + j] == ctx)
                    {
                        if candidate.is_some() {
                            return Err(Error::AmbiguousMatch {
                                user: "Multiple matching contexts found".to_string(),
                                details: format!(
                                    "Multiple matches for context: {:?}",
                                    hunk.context_before
                                ),
                            });
                        }
                        candidate = Some(i);
                    }
                }
                candidate.ok_or_else(|| Error::Apply {
                    user: "Failed to apply patch".to_string(),
                    details: format!("Could not find context: {:?}", hunk.context_before),
                })?
            };

            let deletion_start = hunk_pos + hunk.context_before.len();
            if !hunk.deletions.is_empty() {
                if deletion_start + hunk.deletions.len() > lines.len() {
                    return Err(Error::Apply {
                        user: "Failed to apply patch".to_string(),
                        details: "Deletion extends past end of file".to_string(),
                    });
                }
                for (i, deletion) in hunk.deletions.iter().enumerate() {
                    if lines[deletion_start + i] != deletion {
                        return Err(Error::Apply {
                            user: "Failed to apply patch".to_string(),
                            details: format!(
                                "Deletion mismatch at line {} - expected '{}', found '{}'",
                                deletion_start + i + 1,
                                deletion,
                                lines[deletion_start + i]
                            ),
                        });
                    }
                }
            }

            if pos < hunk_pos {
                result.extend(lines[pos..hunk_pos].iter().map(|s| s.to_string()));
            }
            result.extend(
                hunk.context_before
                    .iter()
                    .enumerate()
                    .map(|(i, _)| lines[hunk_pos + i].to_string()),
            );
            result.extend(hunk.additions.iter().cloned());

            pos = deletion_start + hunk.deletions.len();
        }

        if pos < lines.len() {
            result.extend(lines[pos..].iter().map(|s| s.to_string()));
        }

        let mut output = result.join("\n");
        if !result.is_empty() && input.contains('\n') && input.ends_with('\n') {
            let last_hunk = self.hunks.last().unwrap();
            // Append newline only if the last hunk did not remove the trailing newline.
            if last_hunk.deletions.is_empty()
                || !last_hunk.additions.is_empty()
                || !last_hunk.context_after.is_empty()
            {
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
    // Collect initial matching lines.
    while i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
        current_hunk.context_before.push(old_lines[i].to_string());
        i += 1;
        j += 1;
    }
    let lookahead = 3;
    while i < old_lines.len() || j < new_lines.len() {
        let mut next_match = None;
        for di in 0..=lookahead {
            for dj in 0..=lookahead {
                if di == 0 && dj == 0 {
                    continue;
                }
                if i + di < old_lines.len()
                    && j + dj < new_lines.len()
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
        if let Some((di, dj)) = next_match {
            current_hunk
                .deletions
                .extend(old_lines[i..i + di].iter().map(|s| s.to_string()));
            current_hunk
                .additions
                .extend(new_lines[j..j + dj].iter().map(|s| s.to_string()));
            i += di;
            j += dj;
            let mut matching = 0;
            while i + matching < old_lines.len()
                && j + matching < new_lines.len()
                && old_lines[i + matching] == new_lines[j + matching]
                && matching < lookahead
            {
                matching += 1;
            }
            let matched: Vec<String> = old_lines[i..i + matching]
                .iter()
                .map(|s| s.to_string())
                .collect();
            i += matching;
            j += matching;
            if !current_hunk.deletions.is_empty() || !current_hunk.additions.is_empty() {
                if !current_hunk.deletions.is_empty() && !current_hunk.additions.is_empty() {
                    current_hunk.context_after = Vec::new();
                } else {
                    current_hunk.context_after = matched.clone();
                }
                hunks.push(current_hunk);
                current_hunk = Hunk {
                    context_before: matched,
                    deletions: Vec::new(),
                    additions: Vec::new(),
                    context_after: Vec::new(),
                };
            } else {
                current_hunk.context_before.extend(matched);
            }
        } else {
            // Handle remaining deletions and additions
            current_hunk.deletions.extend(
                old_lines[i..i + (old_lines.len() - i)]
                    .iter()
                    .map(|s| s.to_string()),
            );
            current_hunk.additions.extend(
                new_lines[j..j + (new_lines.len() - j)]
                    .iter()
                    .map(|s| s.to_string()),
            );

            // Add trailing context if both sides have matching content
            let remaining_old = old_lines.len() - (i + current_hunk.deletions.len());
            let remaining_new = new_lines.len() - (j + current_hunk.additions.len());
            let context_size = std::cmp::min(remaining_old, remaining_new);
            if context_size > 0 {
                current_hunk.context_after = old_lines[old_lines.len() - context_size..]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            }
            break;
        }
    }
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
        return Err(Error::Parse {
            user: "Failed to parse diff".to_string(),
            details: "No hunks found in diff".to_string(),
        });
    }

    for line in input.lines() {
        if line.starts_with("@@") {
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

        // Skip headers
        if line.starts_with("---") || line.starts_with("+++") {
            continue;
        }

        // Ensure the line is within a hunk.
        let hunk = current_hunk.as_mut().ok_or_else(|| Error::Parse {
            user: "Failed to parse diff".to_string(),
            details: "Line found outside of hunk".to_string(),
        })?;

        if line.is_empty() {
            continue;
        }

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
            _ => {
                // Lines that don't start with a diff marker are treated as context
                if hunk.deletions.is_empty() && hunk.additions.is_empty() {
                    hunk.context_before.push(line.to_string());
                } else {
                    hunk.context_after.push(line.to_string());
                }
            }
        }
    }

    // Append the final hunk if present.
    if let Some(hunk) = current_hunk.take() {
        hunks.push(hunk);
    }

    Ok(FuDiff { hunks })
}
