Feature: embed shows live ingestion progress on stderr

  As a developer-curator
  I want `mdsearch embed` to report per-file progress on stderr during long ingestion runs
  So that I can tell the tool is working and estimate how far a rebuild is instead of staring at a silent terminal

  Scenario: Per-file progress is shown on stderr for a single collection
    Given a collection named "Notes" has 340 files and a built lexical index
    And the embedding model is available locally
    When I run `mdsearch embed`
    Then stderr shows progress for "Notes" advancing by file, reporting the completed file count against the total
    And the progress reports "Notes" by name

  Scenario: Multi-collection runs report progress per collection
    Given collections "Notes" and "Journal" both have files and built lexical indexes
    And the embedding model is available locally
    When I run `mdsearch embed`
    Then stderr shows progress for "Notes" naming the collection and restarting its file counter
    And stderr shows progress for "Journal" naming the collection and restarting its file counter

  Scenario: An already-current collection produces no progress
    Given a collection named "Notes" is already embedded and current
    When I run `mdsearch embed`
    Then no progress output is shown for "Notes"
    And stdout reports "Notes" as already current

  Scenario: A skipped collection produces no progress
    Given a collection named "Notes" has no built lexical index
    When I run `mdsearch embed`
    Then no progress output is shown for "Notes"
    And stdout reports "Notes" as skipped

  Scenario: The stdout report is unchanged with progress enabled
    Given a collection named "Notes" has files and a built lexical index
    And the embedding model is available locally
    When I run `mdsearch embed`
    Then the stdout report is identical to the pre-progress output for the same input state

  Scenario: The database write phase shows a single status message
    Given a collection named "Notes" has files and a built lexical index
    And the embedding model is available locally
    When I run `mdsearch embed`
    Then the embedding phase shows per-file progress on stderr
    And the database write phase shows a single status message without a progress bar

  Scenario: A failing collection finalizes progress before the failure line
    Given a collection named "Notes" has files and a built lexical index
    And the embedding model is available locally
    And embedding fails partway through "Notes"
    When I run `mdsearch embed`
    Then the progress bar for "Notes" is finalized on stderr
    And stdout reports the per-collection failure line for "Notes"

  Scenario: The --download path is unchanged
    Given a collection named "Notes" has files and a built lexical index
    And the embedding model is not available locally
    When I run `mdsearch embed --download`
    Then the download phase behaves exactly as before
    And once embedding starts, per-file progress is shown on stderr