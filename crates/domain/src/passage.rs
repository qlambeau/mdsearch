//! Passage segmentation for lexical indexing.

use yaml_rust2::Yaml;
use yaml_rust2::YamlLoader;

/// The kind of content a passage contains.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PassageKind {
    /// A body paragraph.
    Body,
    /// The frontmatter `title` field.
    Title,
    /// The frontmatter `tags` field.
    Tags,
    /// The frontmatter `aliases` field.
    Aliases,
    /// The frontmatter `summary` field.
    Summary,
}

impl PassageKind {
    /// Returns the stable database key for this passage kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Title => "title",
            Self::Tags => "tags",
            Self::Aliases => "aliases",
            Self::Summary => "summary",
        }
    }

    /// Reconstructs a passage kind from its stable database key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "body" => Some(Self::Body),
            "title" => Some(Self::Title),
            "tags" => Some(Self::Tags),
            "aliases" => Some(Self::Aliases),
            "summary" => Some(Self::Summary),
            _ => None,
        }
    }
}

/// A single indexed passage of a markdown file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Passage {
    kind: PassageKind,
    text: String,
    byte_offset: usize,
}

impl Passage {
    /// Creates a passage with the given kind, text, and byte offset in its file.
    #[must_use]
    pub const fn new(kind: PassageKind, text: String, byte_offset: usize) -> Self {
        Self {
            kind,
            text,
            byte_offset,
        }
    }

    /// Returns the passage kind.
    #[must_use]
    pub const fn kind(&self) -> PassageKind {
        self.kind
    }

    /// Returns the passage text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the byte offset of the passage in its file.
    #[must_use]
    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }
}

/// Describes a lenient frontmatter handling outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontmatterIssue {
    /// The frontmatter block could not be parsed; the file is indexed body-only.
    Malformed,
}

/// Segments markdown file content into indexed passages.
///
/// Recognized frontmatter fields (`title`, `tags`, `aliases`, `summary`) each
/// become their own passage, and the body is split into one passage per
/// paragraph (contiguous non-blank lines). Each passage records its byte offset
/// in the input content. Frontmatter is optional and lenient: a missing or
/// malformed block never fails, and malformed frontmatter yields body-only
/// passages flagged by [`FrontmatterIssue::Malformed`]. An empty file yields no
/// passages.
#[must_use]
pub fn segment_passages(content: &[u8]) -> (Vec<Passage>, Option<FrontmatterIssue>) {
    let text = String::from_utf8_lossy(content);
    let starts = line_starts(&text);
    let (fields, body_start, issue) = extract_frontmatter(&text, &starts);

    let mut passages = Vec::with_capacity(fields.len() + 1);
    for (kind, value, offset) in fields {
        passages.push(Passage::new(kind, value, offset));
    }
    for (paragraph, offset) in split_paragraphs(&text, body_start) {
        passages.push(Passage::new(PassageKind::Body, paragraph, offset));
    }

    (passages, issue)
}

/// Returns the byte offset at which each line of `text` begins.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

/// Splits a markdown document into its frontmatter fields and body.
///
/// Returns the recognized fields with their byte offsets, the byte offset at
/// which the body begins, and any frontmatter issue.
fn extract_frontmatter(
    text: &str,
    starts: &[usize],
) -> (
    Vec<(PassageKind, String, usize)>,
    usize,
    Option<FrontmatterIssue>,
) {
    let first_line_end = starts.get(1).copied().unwrap_or(text.len());
    let first_line = text.get(0..first_line_end).unwrap_or_default();

    if first_line.trim() != "---" {
        return (Vec::new(), 0, None);
    }

    let Some(closing) = find_closing_line(text, starts) else {
        return (Vec::new(), 0, None);
    };

    let block_start = starts.get(1).copied().unwrap_or(0);
    let block_end = starts.get(closing).copied().unwrap_or(text.len());
    let block = text.get(block_start..block_end).unwrap_or_default();
    let body_start = starts.get(closing + 1).copied().unwrap_or(text.len());

    match YamlLoader::load_from_str(block) {
        Err(_) => (Vec::new(), body_start, Some(FrontmatterIssue::Malformed)),
        Ok(docs) => {
            let fields = docs
                .first()
                .map(|yaml| frontmatter_fields(yaml, text, starts, closing))
                .unwrap_or_default();
            (fields, body_start, None)
        }
    }
}

/// Returns the line index of the closing `---` frontmatter delimiter.
fn find_closing_line(text: &str, starts: &[usize]) -> Option<usize> {
    starts
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, &start)| {
            let end = starts.get(index + 1).copied().unwrap_or(text.len());
            let line = text.get(start..end).unwrap_or_default();
            if line.trim() == "---" {
                Some(index)
            } else {
                None
            }
        })
}

/// Extracts the recognized frontmatter fields in canonical order with offsets.
fn frontmatter_fields(
    yaml: &Yaml,
    text: &str,
    starts: &[usize],
    closing: usize,
) -> Vec<(PassageKind, String, usize)> {
    let mut fields = Vec::new();
    if let Some(value) = field_text(yaml, "title") {
        fields.push((
            PassageKind::Title,
            value,
            field_offset(text, starts, closing, "title"),
        ));
    }
    if let Some(value) = field_text(yaml, "tags") {
        fields.push((
            PassageKind::Tags,
            value,
            field_offset(text, starts, closing, "tags"),
        ));
    }
    if let Some(value) = field_text(yaml, "aliases") {
        fields.push((
            PassageKind::Aliases,
            value,
            field_offset(text, starts, closing, "aliases"),
        ));
    }
    if let Some(value) = field_text(yaml, "summary") {
        fields.push((
            PassageKind::Summary,
            value,
            field_offset(text, starts, closing, "summary"),
        ));
    }
    fields
}

/// Returns the byte offset of the line that declares `key` in the frontmatter.
fn field_offset(text: &str, starts: &[usize], closing: usize, key: &str) -> usize {
    for index in 1..closing {
        let start = starts.get(index).copied().unwrap_or(0);
        let end = starts.get(index + 1).copied().unwrap_or(text.len());
        let line = text.get(start..end).unwrap_or_default().trim_start();
        let is_key = line.len() == key.len() && line == key;
        let is_key_value = line.len() > key.len()
            && line.starts_with(key)
            && line.as_bytes().get(key.len()).copied() == Some(b':');
        if is_key || is_key_value {
            return start;
        }
    }
    0
}

/// Reads a frontmatter field as a non-empty scalar string or joined list.
fn field_text(yaml: &Yaml, key: &str) -> Option<String> {
    let value = &yaml[key];
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    } else if let Some(items) = value.as_vec() {
        let parts = items.iter().filter_map(Yaml::as_str).collect::<Vec<&str>>();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    } else {
        None
    }
}

/// Splits body text starting at `start` into one non-empty paragraph per group.
///
/// Returns each paragraph's trimmed text and the byte offset of its first
/// non-whitespace byte in the original content.
fn split_paragraphs(text: &str, start: usize) -> Vec<(String, usize)> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut paragraph_offset: Option<usize> = None;
    let mut index = start;

    loop {
        let line_end = bytes
            .get(index..)
            .and_then(|rest| rest.iter().position(|&byte| byte == b'\n'))
            .map_or(bytes.len(), |position| index + position);
        let line = text.get(index..line_end).unwrap_or_default();

        if line.trim().is_empty() {
            if let Some(offset) = paragraph_offset.take() {
                push_paragraph(&mut result, text, offset, index);
            }
        } else if paragraph_offset.is_none() {
            let leading = line.len() - line.trim_start().len();
            paragraph_offset = Some(index + leading);
        }

        if line_end == bytes.len() {
            break;
        }
        index = line_end + 1;
    }

    if let Some(offset) = paragraph_offset.take() {
        push_paragraph(&mut result, text, offset, bytes.len());
    }

    result
}

/// Trims and records the paragraph spanning `offset..end` in `text`.
fn push_paragraph(result: &mut Vec<(String, usize)>, text: &str, offset: usize, end: usize) {
    let raw = text.get(offset..end).unwrap_or_default();
    let leading = raw.len() - raw.trim_start().len();
    let trimmed = raw.trim();
    if !trimmed.is_empty() {
        result.push((trimmed.to_owned(), offset + leading));
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::FrontmatterIssue;
    use super::Passage;
    use super::PassageKind;
    use super::segment_passages;

    fn texts(content: &str) -> Vec<String> {
        let (passages, _) = segment_passages(content.as_bytes());
        passages
            .iter()
            .map(|passage| passage.text().to_owned())
            .collect()
    }

    /// Covers: FR-003 — body paragraphs each become one passage.
    #[test]
    fn segments_body_only_content_into_paragraphs() {
        let (passages, issue) = segment_passages(b"alpha\n\nbeta");

        assert_eq!(issue, None);
        assert_eq!(texts("alpha\n\nbeta"), vec!["alpha", "beta"]);
        assert!(
            passages
                .iter()
                .all(|passage| passage.kind() == PassageKind::Body)
        );
    }

    /// Covers: FR-003 — paragraphs are split on one or more blank lines.
    #[rstest]
    #[case("one\n\n\n\ntwo", vec!["one", "two"])]
    #[case("\n\none\n\ntwo\n\n", vec!["one", "two"])]
    #[case("one\n   \ntwo", vec!["one", "two"])]
    #[case("one\r\n\r\ntwo", vec!["one", "two"])]
    #[case("one\nwrapped\n\ntwo", vec!["one\nwrapped", "two"])]
    fn splits_paragraphs_on_blank_lines(#[case] content: &str, #[case] expected: Vec<&str>) {
        let (passages, _) = segment_passages(content.as_bytes());
        let actual = passages.iter().map(Passage::text).collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }

    /// Covers: FR-004 — each recognized frontmatter field is its own passage.
    #[test]
    fn extracts_recognized_frontmatter_fields_as_own_passages() {
        let content = "---\ntitle: My Title\ntags: [rust, cli]\naliases: [mt, my]\nsummary: A summary.\n---\n\none\n\ntwo";
        let (passages, issue) = segment_passages(content.as_bytes());

        assert_eq!(issue, None);
        assert_eq!(
            passages.iter().map(Passage::kind).collect::<Vec<_>>(),
            vec![
                PassageKind::Title,
                PassageKind::Tags,
                PassageKind::Aliases,
                PassageKind::Summary,
                PassageKind::Body,
                PassageKind::Body,
            ]
        );
        assert_eq!(
            passages.iter().map(Passage::text).collect::<Vec<_>>(),
            vec!["My Title", "rust cli", "mt my", "A summary.", "one", "two"]
        );
    }

    /// Covers: FR-004 — scalar list fields are joined into one passage.
    #[test]
    fn accepts_tags_as_a_scalar_list() {
        let content = "---\ntags: rust, cli\n---\n\nbody";
        let (passages, _) = segment_passages(content.as_bytes());

        assert_eq!(
            passages
                .iter()
                .map(|passage| (passage.kind(), passage.text()))
                .collect::<Vec<_>>(),
            vec![
                (PassageKind::Tags, "rust, cli"),
                (PassageKind::Body, "body")
            ]
        );
    }

    /// Covers: FR-006 — malformed frontmatter is flagged and indexed body-only.
    #[test]
    fn flags_malformed_frontmatter_and_indexes_body_only() {
        let content = "---\ntitle: \"unterminated\n: bad: : :\n---\n\nbody";
        let (passages, issue) = segment_passages(content.as_bytes());

        assert_eq!(issue, Some(FrontmatterIssue::Malformed));
        assert_eq!(
            passages.iter().map(Passage::kind).collect::<Vec<_>>(),
            vec![PassageKind::Body]
        );
        assert_eq!(
            passages.iter().map(Passage::text).collect::<Vec<_>>(),
            vec!["body"]
        );
    }

    /// Covers: FR-005 — a file without frontmatter is indexed body-only.
    #[test]
    fn treats_missing_frontmatter_as_body_only() {
        let (passages, issue) = segment_passages(b"one\n\ntwo");

        assert_eq!(issue, None);
        assert!(
            passages
                .iter()
                .all(|passage| passage.kind() == PassageKind::Body)
        );
    }

    /// Covers: the lenient boundary — a dangling opening delimiter is body text.
    #[test]
    fn treats_missing_closing_delimiter_as_body_only() {
        let content = "---\ntitle: x";
        let (passages, issue) = segment_passages(content.as_bytes());

        assert_eq!(issue, None);
        assert_eq!(passages.len(), 1);
        assert_eq!(
            passages.iter().map(Passage::kind).collect::<Vec<_>>(),
            vec![PassageKind::Body]
        );
    }

    /// Covers: FR-007 — an empty file contributes no passages.
    #[test]
    fn empty_content_produces_no_passages() {
        let (passages, issue) = segment_passages(b"");

        assert!(passages.is_empty());
        assert_eq!(issue, None);
    }

    /// Covers: FR-007 — a delimiter-only file contributes no passages.
    #[test]
    fn delimiter_only_content_produces_no_passages() {
        let (passages, issue) = segment_passages(b"---\n---\n");

        assert!(passages.is_empty());
        assert_eq!(issue, None);
    }

    /// Covers: FR-004 — a file with fields but no body yields only field passages.
    #[test]
    fn frontmatter_only_content_yields_field_passages() {
        let content = "---\ntitle: Only Title\n---\n";
        let (passages, issue) = segment_passages(content.as_bytes());

        assert_eq!(issue, None);
        assert_eq!(passages.len(), 1);
        assert_eq!(
            passages.iter().map(Passage::kind).collect::<Vec<_>>(),
            vec![PassageKind::Title]
        );
    }

    /// Covers: the stable database keys for each passage kind.
    #[test]
    fn passage_kinds_have_stable_keys() {
        assert_eq!(PassageKind::Body.as_str(), "body");
        assert_eq!(PassageKind::Title.as_str(), "title");
        assert_eq!(PassageKind::Tags.as_str(), "tags");
        assert_eq!(PassageKind::Aliases.as_str(), "aliases");
        assert_eq!(PassageKind::Summary.as_str(), "summary");
    }

    /// Covers: FR-007 — body paragraphs carry their byte offset in the file.
    #[test]
    fn body_passages_record_byte_offsets() {
        let (passages, _) = segment_passages(b"alpha\n\nbeta");

        assert_eq!(
            passages
                .iter()
                .map(|passage| (passage.text().to_owned(), passage.byte_offset()))
                .collect::<Vec<_>>(),
            vec![("alpha".to_owned(), 0), ("beta".to_owned(), 7)]
        );
    }

    /// Covers: FR-007 — repeated paragraphs get distinct offsets.
    #[test]
    fn repeated_paragraphs_get_distinct_offsets() {
        let (passages, _) = segment_passages(b"hello\n\nhello");

        let offsets = passages
            .iter()
            .map(Passage::byte_offset)
            .collect::<Vec<_>>();
        assert_eq!(offsets, vec![0, 7]);
    }

    /// Covers: FR-007 — CRLF content records correct offsets.
    #[test]
    fn crlf_content_records_correct_offsets() {
        let (passages, _) = segment_passages(b"one\r\n\r\ntwo");

        assert_eq!(
            passages
                .iter()
                .map(|passage| (passage.text().to_owned(), passage.byte_offset()))
                .collect::<Vec<_>>(),
            vec![("one".to_owned(), 0), ("two".to_owned(), 7)]
        );
    }

    /// Covers: FR-007 — frontmatter fields carry their key-line offset.
    #[test]
    fn frontmatter_fields_record_key_line_offsets() -> Result<(), Box<dyn std::error::Error>> {
        let content = "---\ntags: [rust]\ntitle: Notes\n---\n\nbody";
        let (passages, _) = segment_passages(content.as_bytes());

        let tags = passages
            .iter()
            .find(|passage| passage.kind() == PassageKind::Tags)
            .ok_or_else(|| std::io::Error::other("expected a tags passage"))?;
        let title = passages
            .iter()
            .find(|passage| passage.kind() == PassageKind::Title)
            .ok_or_else(|| std::io::Error::other("expected a title passage"))?;

        assert_eq!(tags.byte_offset(), 4);
        assert_eq!(title.byte_offset(), 17);
        assert!(
            passages
                .iter()
                .any(|passage| passage.kind() == PassageKind::Body)
        );

        Ok(())
    }

    /// Covers: FR-007 — the byte offset points at the first non-whitespace byte.
    #[test]
    fn body_paragraph_offset_skips_leading_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        let (passages, _) = segment_passages(b"  indented paragraph\n\nnext");

        let first = passages
            .iter()
            .find(|passage| passage.kind() == PassageKind::Body)
            .ok_or_else(|| std::io::Error::other("expected a body passage"))?;
        assert_eq!(first.byte_offset(), 2);

        Ok(())
    }

    /// Covers: `from_key` round-trips every kind from its stable key.
    #[test]
    fn from_key_round_trips_every_kind() {
        assert_eq!(PassageKind::from_key("body"), Some(PassageKind::Body));
        assert_eq!(PassageKind::from_key("title"), Some(PassageKind::Title));
        assert_eq!(PassageKind::from_key("tags"), Some(PassageKind::Tags));
        assert_eq!(PassageKind::from_key("aliases"), Some(PassageKind::Aliases));
        assert_eq!(PassageKind::from_key("summary"), Some(PassageKind::Summary));
    }

    /// Covers: `from_key` rejects unknown keys.
    #[test]
    fn from_key_rejects_unknown_keys() {
        assert_eq!(PassageKind::from_key("frontmatter"), None);
        assert_eq!(PassageKind::from_key(""), None);
        assert_eq!(PassageKind::from_key("BODY"), None);
    }

    proptest! {
        /// Covers: FR-003 — segmentation is deterministic for arbitrary content.
        #[test]
        fn segmentation_is_deterministic(content in prop::collection::vec(any::<u8>(), 0..4096)) {
            let first = segment_passages(&content);
            let second = segment_passages(&content);

            prop_assert_eq!(first, second);
        }
    }

    proptest! {
        /// Covers: FR-003 — a body with N paragraphs yields N body passages.
        #[test]
        fn paragraph_count_matches_blank_line_separated_groups(paragraphs in prop::collection::vec(prop::collection::vec("[a-zA-Z0-9]{1,20}", 1..5), 0..20)) {
            let content = paragraphs.iter().map(|lines| lines.join("\n")).collect::<Vec<_>>().join("\n\n");
            let (passages, _) = segment_passages(content.as_bytes());
            let body_count = passages.iter().filter(|passage| passage.kind() == PassageKind::Body).count();

            prop_assert_eq!(body_count, paragraphs.len());
        }
    }
}
