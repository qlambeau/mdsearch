Feature: Add markdown files to a collection

  As a developer-curator
  I want to add markdown files to a collection
  So that the collection has content that can later be indexed and searched

  Scenario: Add markdown files from a directory recursively
    Given a collection named "Notes" exists
    And a directory "vault" contains "a.md", "sub/b.md", and "readme.txt"
    When I run `mdsearch collection add Notes vault`
    Then the human-readable output reports 2 files added

  Scenario: Add a single markdown file
    Given a collection named "Notes" exists
    And "notes.md" is a readable markdown file
    When I run `mdsearch collection add Notes notes.md`
    Then the human-readable output reports 1 file added

  Scenario: Re-adding a file replaces its content without duplicating
    Given a collection named "Notes" exists
    And "notes.md" was already added to "Notes"
    When I run `mdsearch collection add Notes notes.md`
    Then the human-readable output reports 1 file added
    And no duplicate file is created for "notes.md"

  Scenario: Fail without ingesting when a path is unreadable
    Given a collection named "Notes" exists
    And "missing.md" does not exist
    When I run `mdsearch collection add Notes missing.md`
    Then the operation fails
    And nothing is ingested

  Scenario: Skip unreadable paths with --force
    Given a collection named "Notes" exists
    And "missing.md" does not exist
    And "notes.md" is a readable markdown file
    When I run `mdsearch collection add Notes missing.md notes.md --force`
    Then the human-readable output reports 1 file added and 1 skipped

  Scenario: Report a missing collection
    Given no collection named "Notes" exists
    When I run `mdsearch collection add Notes notes.md`
    Then the operation fails
    And the output communicates that the collection was not found

  Scenario: Report a missing database
    Given no database exists at "/tmp/mdsearch-add/missing/collections.db"
    When I run `mdsearch collection add Notes notes.md --database "/tmp/mdsearch-add/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-add/missing/collections.db"

  Scenario: Add files to an explicitly selected database
    Given a collection named "Notes" exists at "/custom/collections.db"
    And "notes.md" is a readable markdown file
    When I run `mdsearch collection add Notes notes.md --database "/custom/collections.db"`
    Then the human-readable output reports 1 file added
