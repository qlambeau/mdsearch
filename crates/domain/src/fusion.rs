//! Reciprocal Rank Fusion and free-text query mapping for hybrid search.

use crate::file_id::FileId;
use crate::passage::PassageKind;

/// The stable logical identity of a passage used as the fusion key.
///
/// `(file, kind, position)` identifies a passage independently of the physical
/// FTS5 rowid, so lexical and semantic candidates for the same passage map to
/// the same key.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PassageKey {
    file: FileId,
    kind: PassageKind,
    position: usize,
}

impl PassageKey {
    /// Creates a passage key from its logical identity components.
    #[must_use]
    pub const fn new(file: FileId, kind: PassageKind, position: usize) -> Self {
        Self {
            file,
            kind,
            position,
        }
    }

    /// Returns the owning file's ID.
    #[must_use]
    pub const fn file(self) -> FileId {
        self.file
    }

    /// Returns the passage kind.
    #[must_use]
    pub const fn kind(self) -> PassageKind {
        self.kind
    }

    /// Returns the passage's position within its file.
    #[must_use]
    pub const fn position(self) -> usize {
        self.position
    }
}

/// A fused passage with its Reciprocal Rank Fusion score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FusedRank {
    key: PassageKey,
    score: f64,
}

impl FusedRank {
    /// Creates a fused rank from its passage key and RRF score.
    #[must_use]
    pub const fn new(key: PassageKey, score: f64) -> Self {
        Self { key, score }
    }

    /// Returns the fused passage key.
    #[must_use]
    pub const fn key(self) -> PassageKey {
        self.key
    }

    /// Returns the fused RRF score.
    #[must_use]
    pub const fn score(self) -> f64 {
        self.score
    }
}

/// The default Reciprocal Rank Fusion constant.
///
/// This value is tunable through the ADR-004 evaluation framework.
pub const DEFAULT_RRF_K: f64 = 60.0;

/// Fuses two ranked candidate lists with Reciprocal Rank Fusion.
///
/// Each list is in descending relevance order; the fused score of a passage is
/// the sum of `1 / (k + rank)` over every list in which it appears, where
/// `rank` is the 1-based position in that list. The result is sorted by fused
/// score descending, with ties broken by the passage key's total order.
///
/// `k` must be positive; the shared [`DEFAULT_RRF_K`] satisfies this. Ranks
/// are 1-based, so the denominator never reaches zero for a non-negative `k`.
#[must_use]
pub fn reciprocal_rank_fusion(
    lexical: &[PassageKey],
    semantic: &[PassageKey],
    k: f64,
) -> Vec<FusedRank> {
    let mut scores: Vec<(PassageKey, f64)> = Vec::new();
    for list in [lexical, semantic] {
        for (index, key) in list.iter().enumerate() {
            let rank = match u32::try_from(index) {
                Ok(value) => f64::from(value) + 1.0,
                Err(_) => f64::from(u32::MAX) + 1.0,
            };
            let contribution = 1.0 / (k + rank);
            match scores.iter_mut().find(|(stored, _)| stored == key) {
                Some((_, score)) => *score += contribution,
                None => scores.push((*key, contribution)),
            }
        }
    }

    scores.sort_by(|(left_key, left_score), (right_key, right_score)| {
        right_score
            .partial_cmp(left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left_key.cmp(right_key))
    });

    scores
        .into_iter()
        .map(|(key, score)| FusedRank::new(key, score))
        .collect()
}

/// Maps free-text query terms to an FTS5 `AND`-joined quoted-term match.
///
/// Each whitespace-separated term is wrapped in double quotes so FTS5 operator
/// characters (`AND`, `OR`, `NOT`, quotes, `prefix*`) are treated as literal
/// text, and the terms are joined with ` AND ` so every term must match. A
/// term containing a double quote has it escaped by doubling.
///
/// Returns `None` when the query contains no terms.
#[must_use]
pub fn free_text_to_fts5(query: &str) -> Option<String> {
    let terms = query.split_whitespace().map(quote_term).collect::<Vec<_>>();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

/// Quotes a single FTS5 term, escaping embedded double quotes by doubling.
fn quote_term(term: &str) -> String {
    let escaped = term.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::DEFAULT_RRF_K;
    use super::FusedRank;
    use super::PassageKey;
    use super::free_text_to_fts5;
    use super::reciprocal_rank_fusion;
    use crate::FileId;
    use crate::PassageKind;

    fn key(
        file: u64,
        kind: PassageKind,
        position: usize,
    ) -> Result<PassageKey, crate::FileIdError> {
        Ok(PassageKey::new(FileId::try_new(file)?, kind, position))
    }

    /// Covers: FR-001 — a passage matched only by the lexical leg keeps its score.
    #[test]
    fn a_lexical_only_passage_keeps_its_score() -> Result<(), Box<dyn std::error::Error>> {
        let a = key(1, PassageKind::Body, 0)?;
        let fused = reciprocal_rank_fusion(&[a], &[], DEFAULT_RRF_K);

        assert_eq!(fused.len(), 1);
        let first = fused
            .first()
            .ok_or_else(|| std::io::Error::other("expected one fused rank"))?;
        assert_eq!(first.key(), a);
        assert!((first.score() - (1.0 / (DEFAULT_RRF_K + 1.0))).abs() < 1e-9);

        Ok(())
    }

    /// Covers: FR-001 — a passage matched by both legs sums both contributions.
    #[test]
    fn a_both_leg_passage_sums_both_contributions() -> Result<(), Box<dyn std::error::Error>> {
        let a = key(1, PassageKind::Body, 0)?;
        let fused = reciprocal_rank_fusion(&[a], &[a], DEFAULT_RRF_K);

        assert_eq!(fused.len(), 1);
        let first = fused
            .first()
            .ok_or_else(|| std::io::Error::other("expected one fused rank"))?;
        let expected = 2.0 / (DEFAULT_RRF_K + 1.0);
        assert!((first.score() - expected).abs() < 1e-9);

        Ok(())
    }

    /// Covers: FR-001 — a passage ranked first in both legs outranks all others.
    #[test]
    fn a_both_leg_top_passage_outranks_single_leg_passages()
    -> Result<(), Box<dyn std::error::Error>> {
        let both = key(1, PassageKind::Body, 0)?;
        let lexical_only = key(2, PassageKind::Body, 0)?;
        let semantic_only = key(3, PassageKind::Body, 0)?;
        let fused =
            reciprocal_rank_fusion(&[both, lexical_only], &[both, semantic_only], DEFAULT_RRF_K);

        let first = fused
            .first()
            .ok_or_else(|| std::io::Error::other("expected a first fused rank"))?;
        assert_eq!(first.key(), both);
        assert_eq!(fused.len(), 3);

        Ok(())
    }

    /// Covers: FR-001 — ranks inside a list contribute by position.
    #[test]
    fn rank_position_within_a_list_contributes_to_the_score()
    -> Result<(), Box<dyn std::error::Error>> {
        let first = key(1, PassageKind::Body, 0)?;
        let second = key(2, PassageKind::Body, 0)?;
        let fused = reciprocal_rank_fusion(&[first, second], &[], DEFAULT_RRF_K);

        let first_rank = fused
            .first()
            .ok_or_else(|| std::io::Error::other("expected a first fused rank"))?;
        let second_rank = fused
            .get(1)
            .ok_or_else(|| std::io::Error::other("expected a second fused rank"))?;
        assert!(first_rank.score() > second_rank.score());

        Ok(())
    }

    /// Covers: FR-014 — equal scores tie-break by passage key order.
    #[test]
    fn equal_scores_tie_break_by_key_order() -> Result<(), Box<dyn std::error::Error>> {
        let low = key(1, PassageKind::Body, 0)?;
        let high = key(2, PassageKind::Body, 0)?;
        let fused = reciprocal_rank_fusion(&[low, high], &[high, low], DEFAULT_RRF_K);

        assert_eq!(fused.len(), 2);
        let first = fused
            .first()
            .ok_or_else(|| std::io::Error::other("expected a first fused rank"))?;
        let second = fused
            .get(1)
            .ok_or_else(|| std::io::Error::other("expected a second fused rank"))?;
        assert!(first.key() < second.key());

        Ok(())
    }

    /// Covers: FR-001 — fusion is deterministic for the same inputs.
    #[test]
    fn fusion_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let lexical = [key(3, PassageKind::Body, 0)?, key(1, PassageKind::Body, 0)?];
        let semantic = [key(1, PassageKind::Body, 0)?, key(2, PassageKind::Body, 0)?];

        let first = reciprocal_rank_fusion(&lexical, &semantic, DEFAULT_RRF_K);
        let second = reciprocal_rank_fusion(&lexical, &semantic, DEFAULT_RRF_K);

        assert_eq!(first, second);

        Ok(())
    }

    /// Covers: FR-001 — empty lists produce an empty fusion.
    #[test]
    fn empty_lists_produce_empty_fusion() {
        let fused = reciprocal_rank_fusion(&[], &[], DEFAULT_RRF_K);

        assert!(fused.is_empty());
    }

    /// Covers: FR-002 — the mapper quotes terms and joins with AND.
    #[test]
    fn maps_free_text_to_and_joined_quoted_terms() {
        assert_eq!(
            free_text_to_fts5("borrowing rust").as_deref(),
            Some("\"borrowing\" AND \"rust\"")
        );
    }

    /// Covers: FR-002 — operator characters become literal quoted text.
    #[rstest]
    #[case("a AND b", "\"a\" AND \"AND\" AND \"b\"")]
    #[case("\"rust ownership\"", "\"\"\"rust\" AND \"ownership\"\"\"")]
    #[case("prefix*", "\"prefix*\"")]
    fn neutralizes_fts5_operator_characters(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(free_text_to_fts5(query).as_deref(), Some(expected));
    }

    /// Covers: FR-002 — whitespace is collapsed.
    #[test]
    fn collapses_whitespace() {
        assert_eq!(
            free_text_to_fts5("  borrowing   rust  ").as_deref(),
            Some("\"borrowing\" AND \"rust\"")
        );
    }

    /// Covers: FR-002 — a query with no terms maps to none.
    #[rstest]
    #[case("")]
    #[case("   ")]
    fn no_terms_maps_to_none(#[case] query: &str) {
        assert_eq!(free_text_to_fts5(query), None);
    }

    proptest! {
        /// Covers: FR-001 — fusion is deterministic for arbitrary ranked lists.
        #[test]
        fn fusion_is_deterministic_for_arbitrary_lists(
            lexical in proptest::collection::vec(1u64..100, 0..20),
            semantic in proptest::collection::vec(1u64..100, 0..20),
        ) {
            let lexical = lexical
                .into_iter()
                .map(|id| {
                    FileId::try_new(id)
                        .map(|file| PassageKey::new(file, PassageKind::Body, 0))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
            let semantic = semantic
                .into_iter()
                .map(|id| {
                    FileId::try_new(id)
                        .map(|file| PassageKey::new(file, PassageKind::Body, 0))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;

            let first = reciprocal_rank_fusion(&lexical, &semantic, DEFAULT_RRF_K);
            let second = reciprocal_rank_fusion(&lexical, &semantic, DEFAULT_RRF_K);
            let count = first.len();

            prop_assert_eq!(first, second);
            prop_assert!(count <= lexical.len() + semantic.len());
        }
    }

    /// Guards: the fused rank exposes its key and score.
    #[test]
    fn fused_rank_exposes_key_and_score() -> Result<(), Box<dyn std::error::Error>> {
        let passage = key(7, PassageKind::Title, 1)?;
        let rank = FusedRank::new(passage, 1.5);

        assert_eq!(rank.key(), passage);
        assert!((rank.score() - 1.5).abs() < 1e-9);

        Ok(())
    }

    /// Guards: the passage key exposes its identity components.
    #[test]
    fn passage_key_exposes_identity_components() -> Result<(), Box<dyn std::error::Error>> {
        let passage = key(7, PassageKind::Title, 1)?;

        assert_eq!(passage.file(), FileId::try_new(7)?);
        assert_eq!(passage.kind(), PassageKind::Title);
        assert_eq!(passage.position(), 1);

        Ok(())
    }
}
