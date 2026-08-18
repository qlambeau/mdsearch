Feature: Hybrid search with lexical-semantic fusion and cross-encoder re-ranking

  As a developer-curator and coding-agent harness
  I want `mdsearch hybrid QUERY` to fuse lexical and semantic results and re-rank them with a local cross-encoder
  So that conceptual queries that keywords miss are answered with the best ordering

  Scenario: Hybrid search returns a single fused ranked list
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And both indexes contain a passage about "borrowing"
    And the re-ranker assets are cached locally
    When I run `mdsearch hybrid borrowing`
    Then the hybrid search succeeds
    And ranked passage blocks are returned in re-ranker order
    And each block reports the re-ranker score

  Scenario: A both-leg match outranks single-leg matches
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And a passage matches both the lexical and semantic legs for "borrowing"
    And another passage matches only the semantic leg for "borrowing"
    And the re-ranker assets are cached locally
    When I run `mdsearch hybrid borrowing`
    Then the both-leg passage is ranked above the single-leg passage

  Scenario: --no-rerank orders results by the fused RRF score
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And the re-ranker assets are cached locally
    When I run `mdsearch hybrid borrowing --no-rerank`
    Then ranked passage blocks are returned in fused score order
    And each block reports the fused score

  Scenario: An uncached re-ranker falls back to RRF-only with a warning
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And the re-ranker assets are not cached locally
    When I run `mdsearch hybrid borrowing`
    Then the hybrid search succeeds
    And results are ordered by the fused score
    And the output warns that re-ranking was skipped

  Scenario: --no-rerank with an uncached re-ranker produces no warning
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And the re-ranker assets are not cached locally
    When I run `mdsearch hybrid borrowing --no-rerank`
    Then the hybrid search succeeds
    And the output does not warn about re-ranking

  Scenario: A collection without a semantic index contributes lexical results
    Given a collection named "Notes" has a built lexical index but no semantic index
    And "Notes" contains a passage about "borrowing"
    When I run `mdsearch hybrid borrowing`
    Then the hybrid search succeeds
    And the lexical passage from "Notes" is returned

  Scenario: Fail when an in-scope semantic index is stale
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And the stored files changed after the semantic index was built
    When I run `mdsearch hybrid borrowing`
    Then the operation fails
    And the output directs me to run `mdsearch embed`

  Scenario: Restrict a hybrid search to one collection
    Given collections "Notes" and "Archive" have built lexical and semantic indexes
    When I run `mdsearch hybrid borrowing --collection Notes`
    Then only passages from "Notes" are returned

  Scenario: Report a missing collection for --collection
    Given no collection named "Journal" exists
    When I run `mdsearch hybrid borrowing --collection Journal`
    Then the operation fails
    And the output communicates that the collection was not found

  Scenario: Report an unbuilt lexical index for --collection
    Given a collection named "Notes" stores files but has no built lexical index
    When I run `mdsearch hybrid borrowing --collection Notes`
    Then the operation fails
    And the output communicates that the index is not built

  Scenario: Skip unbuilt and empty collections when searching all
    Given a collection named "Notes" has a built lexical and semantic index
    And a collection named "Draft" stores files but has no built lexical index
    And a collection named "Empty" has no stored files
    When I run `mdsearch hybrid borrowing`
    Then the hybrid search succeeds
    And passages from "Notes" are returned

  Scenario: Cap results with --limit and report the shown count
    Given a built collection matches more than 5 passages for "borrowing"
    When I run `mdsearch hybrid borrowing --limit 5`
    Then at most 5 passage blocks are shown
    And the summary reports 5 results

  Scenario: Reject an out-of-range --limit
    When I run `mdsearch hybrid borrowing --limit 200`
    Then the operation fails

  Scenario: Fail on an empty query
    When I run `mdsearch hybrid ""`
    Then the operation fails

  Scenario: Produce empty output when nothing matches
    Given a built collection exists
    When I run `mdsearch hybrid zzznotaword`
    Then the output is empty

  Scenario: Emit JSON with per-leg, fused, and re-ranker scores
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And a passage matches "borrowing" on both legs
    And the re-ranker assets are cached locally
    When I run `mdsearch hybrid borrowing --json`
    Then the output is valid JSON
    And each result includes the re-ranker score, the fused score, the BM25 score, and the cosine similarity
    And each result includes the collection, file path, passage kind, text, and position

  Scenario: Report a missing database without creating a file
    Given no database exists at "/tmp/mdsearch-hybrid/missing/collections.db"
    When I run `mdsearch hybrid borrowing --database "/tmp/mdsearch-hybrid/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-hybrid/missing/collections.db"
