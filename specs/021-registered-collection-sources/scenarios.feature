# parent: US-021
# status: approved

Feature: Registered collection sources
  Developer-curators can register Markdown sources and refresh collections
  without repeating source paths.

  Scenario: Creating a collection registers sources without indexing
    Given a Markdown directory containing "a.md"
    When I create the collection "Notes" with that directory as a source
    Then the canonical directory source is stored for "Notes"
    And "a.md" is not indexed until the collection is updated

  Scenario: Configuring sources replaces the registered source set
    Given collection "Notes" has source "vault/old"
    When I configure "Notes" with source "vault/new"
    Then "vault/new" is registered for "Notes"
    And "vault/old" is no longer registered for "Notes"

  Scenario Outline: Collection listing reports canonical registered sources
    Given collection "Notes" has registered sources "vault/notes" and "archive.md"
    When I list collections in "<format>" format
    Then the listing includes collection "Notes"
    And the listing includes the canonical paths of both registered sources

    Examples:
      | format  |
      | human   |
      | JSON    |

  Scenario: Update discovers new Markdown files
    Given collection "Notes" has a registered directory containing "a.md"
    And "b.md" is added to that directory after the last update
    When I update collection "Notes"
    Then both "a.md" and "b.md" are stored and indexed

  Scenario: Update reconciles edits and deletions from registered sources
    Given collection "Notes" was updated from a registered directory
    And "a.md" was edited and "b.md" was deleted from that directory
    When I update collection "Notes"
    Then the stored content for "a.md" matches the edited file
    And "b.md" and its index entries are removed

  Scenario: Overlapping sources process a file once
    Given collection "Notes" has registered sources "vault" and "vault/subdir"
    And "vault/subdir/a.md" is discovered by both sources
    When I update collection "Notes"
    Then "a.md" is stored and indexed once

  Scenario: Removing a source deletes files exclusive to that source
    Given collection "Notes" has sources "vault/first" and "vault/second"
    And "first/a.md" is stored only from "vault/first"
    And "second/b.md" is stored and found from "vault/second"
    When I replace the sources with only "vault/second" and update collection "Notes"
    Then "first/a.md" and its index entries are removed
    And "second/b.md" remains stored and indexed

  Scenario: An inaccessible directory aborts the collection update
    Given collection "Notes" has an existing stored file and built indexes
    And one registered directory source is inaccessible
    When I update collection "Notes"
    Then the operation fails
    And the previously stored content and indexes for "Notes" remain unchanged

  Scenario: Skipping an unreadable file preserves its previous content
    Given collection "Notes" has a stored file that is now unreadable
    When I update collection "Notes" with "--skip-unreadable"
    Then the file is reported as skipped
    And the previously stored content remains unchanged

  Scenario: A migrated collection is not assigned guessed sources
    Given collection "Notes" has stored files but no registered sources
    When I update collection "Notes"
    Then the operation fails with guidance to register sources
    And the stored files remain searchable

  Scenario Outline: Update commits content and indexes atomically
    Given collection "Notes" has stored files and built lexical and graph indexes
    And a "<stage>" index build fails during update
    When I update collection "Notes"
    Then the operation fails
    And the previous stored files and lexical and graph indexes remain unchanged

    Examples:
      | stage   |
      | lexical |
      | graph   |

  Scenario: Update all continues after an individual collection failure
    Given collections "Notes" and "Archive" both have registered sources
    And a source directory for "Notes" is inaccessible
    When I update all collections
    Then "Archive" is updated successfully
    And the outcomes for both collections are reported
    And the command exits unsuccessfully
