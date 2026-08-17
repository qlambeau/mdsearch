Feature: Build the lexical index during collection update

  As a developer-curator
  I want `collection update` to build and keep current a lexical (BM25-style) index, and to check its status
  So that the collection is ready for lexical search once the search command is added

  Scenario: Adding files alone does not build the index
    Given a collection named "Notes" stores "a.md"
    And the collection was never updated
    When I run `mdsearch index status`
    Then the report for "Notes" shows state "not built"

  Scenario: Update builds the index and counts passages
    Given a collection named "Notes" stores "a.md" with 3 body paragraphs, a title, and tags
    When I run `mdsearch collection update Notes a.md`
    Then the update succeeds
    And `mdsearch index status` reports "Notes" as "built" with 1 file and 5 passages
    And a build timestamp is recorded for "Notes"

  Scenario: Every recognized frontmatter field becomes its own passage
    Given a collection named "Notes" stores "f.md" with 2 body paragraphs, a title, tags, aliases, and a summary
    When I run `mdsearch collection update Notes f.md`
    Then `mdsearch index status` reports "Notes" as "built" with 6 passages

  Scenario: Update refreshes the index after an edit
    Given a collection named "Notes" stores "a.md" with 3 body paragraphs, a title, and tags
    And the index for "Notes" was built
    And "a.md" on disk now has 5 body paragraphs
    When I run `mdsearch collection update Notes a.md`
    Then `mdsearch index status` reports "Notes" as "built" with 7 passages
    And the build timestamp for "Notes" is refreshed

  Scenario: Update removes passages of a deleted file
    Given a collection named "Notes" stores "a.md" and "b.md"
    And the index for "Notes" was built with passages from both files
    And "b.md" no longer exists on disk
    When I run `mdsearch collection update Notes a.md`
    Then `mdsearch index status` reports "Notes" as "built" with 1 file
    And no passages from "b.md" are counted

  Scenario: Malformed frontmatter is indexed body-only and reported
    Given a collection named "Notes" stores "c.md" with malformed frontmatter and 2 body paragraphs
    When I run `mdsearch collection update Notes c.md`
    Then the update succeeds
    And the update output reports the malformed frontmatter
    And `mdsearch index status` reports "Notes" with 2 passages from "c.md"

  Scenario: Files without frontmatter are indexed by their body
    Given a collection named "Notes" stores "d.md" with 2 paragraphs and no frontmatter
    When I run `mdsearch collection update Notes d.md`
    Then the update succeeds
    And `mdsearch index status` reports "Notes" as "built" with exactly 2 passages

  Scenario: Empty files contribute no passages
    Given a collection named "Notes" stores "e.md" with no content
    When I run `mdsearch collection update Notes e.md`
    Then the update succeeds
    And `mdsearch index status` reports "Notes" as "built" with 0 passages

  Scenario: Index build failure fails the update atomically
    Given a collection named "Notes" stores "a.md"
    And the index for "Notes" was previously built
    And a storage error occurs while building the index
    When I run `mdsearch collection update Notes a.md`
    Then the operation fails
    And the file changes are not committed
    And the previous index state for "Notes" is unchanged

  Scenario: Update --all rebuilds the index for every collection
    Given collections "Notes" and "Archive" each store files
    When I run `mdsearch collection update --all`
    Then `mdsearch index status` reports both "Notes" and "Archive" as "built"

  Scenario: Report a missing database without creating a file
    Given no database exists at "/tmp/mdsearch-index/missing/collections.db"
    When I run `mdsearch index status --database "/tmp/mdsearch-index/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-index/missing/collections.db"

  Scenario: A fresh database with no collections produces empty output
    Given a database exists at "/tmp/mdsearch-index/empty/collections.db" with no collections
    When I run `mdsearch index status --database "/tmp/mdsearch-index/empty/collections.db"`
    Then the output is empty