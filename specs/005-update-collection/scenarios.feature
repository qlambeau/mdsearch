Feature: Update a collection

  As a developer-curator
  I want to update a collection
  So that its stored files reflect the current on-disk state after files were added, modified, or deleted

  Scenario: Update ingests newly added files
    Given a collection named "Notes" stores "a.md"
    And a directory "vault" contains "a.md" and a new "b.md"
    When I run `mdsearch collection update Notes vault`
    Then the human-readable output reports 1 added file

  Scenario: Update re-ingests a modified file
    Given a collection named "Notes" stores "a.md" with content "old"
    And "a.md" on disk now contains "new"
    When I run `mdsearch collection update Notes a.md`
    Then the human-readable output reports 1 modified file

  Scenario: Update removes a deleted file
    Given a collection named "Notes" stores "a.md"
    And "a.md" no longer exists on disk
    When I run `mdsearch collection update Notes vault`
    Then the human-readable output reports 1 deleted file

  Scenario: Update leaves an unchanged file as-is
    Given a collection named "Notes" stores "a.md" with content "same"
    And "a.md" on disk still contains "same"
    When I run `mdsearch collection update Notes a.md`
    Then the human-readable output reports 0 added, 0 modified, and 0 deleted files

  Scenario: Update all collections
    Given collections "Notes" and "Archive" each store files
    And some stored files were modified or deleted on disk
    When I run `mdsearch collection update --all`
    Then the human-readable output reports one line per collection

  Scenario: Fail without changing anything when a path is unreadable
    Given a collection named "Notes" stores "a.md"
    And "missing.md" does not exist
    When I run `mdsearch collection update Notes missing.md`
    Then the operation fails
    And nothing is added, modified, or deleted

  Scenario: Skip unreadable paths with --force
    Given a collection named "Notes" stores "a.md"
    And "missing.md" does not exist
    And "b.md" is a new markdown file
    When I run `mdsearch collection update Notes missing.md b.md --force`
    Then the human-readable output reports 1 added file and 1 skipped

  Scenario: Report a missing collection
    Given no collection named "Notes" exists
    When I run `mdsearch collection update Notes a.md`
    Then the operation fails
    And the output communicates that the collection was not found

  Scenario: Report a missing database
    Given no database exists at "/tmp/mdsearch-update/missing/collections.db"
    When I run `mdsearch collection update Notes a.md --database "/tmp/mdsearch-update/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-update/missing/collections.db"
