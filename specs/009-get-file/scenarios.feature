Feature: Retrieve a complete file by name or ID

  As a developer-curator and coding-agent harness
  I want to retrieve a complete stored file from a collection by name or ID
  So that I can fetch the full source content without searching

  Scenario: Retrieve a file by its canonical path
    Given a collection named "Notes" stores "/vault/notes.md" with content "alpha"
    When I run `mdsearch get Notes /vault/notes.md`
    Then the raw content "alpha" is printed to stdout

  Scenario: Retrieve a file by a unique basename
    Given a collection named "Notes" stores "/vault/notes.md" with content "alpha"
    When I run `mdsearch get Notes notes.md`
    Then the raw content "alpha" is printed to stdout

  Scenario: Retrieve a file by its indexing-assigned ID
    Given a collection named "Notes" stores a file with ID 42 and content "alpha"
    When I run `mdsearch get Notes 42`
    Then the raw content "alpha" is printed to stdout

  Scenario: Report an ambiguous basename with candidates
    Given a collection named "Notes" stores "/a/x.md" and "/b/x.md"
    When I run `mdsearch get Notes x.md`
    Then the operation fails
    And the output lists both candidate paths

  Scenario: Report a file not found by name
    Given a collection named "Notes" stores "/vault/notes.md"
    When I run `mdsearch get Notes missing.md`
    Then the operation fails
    And the output communicates that the file was not found

  Scenario: Report a file not found by ID
    Given a collection named "Notes" stores a file with ID 42
    When I run `mdsearch get Notes 999`
    Then the operation fails
    And the output communicates that the file was not found

  Scenario: Report a missing collection
    Given no collection named "Journal" exists
    When I run `mdsearch get Journal notes.md`
    Then the operation fails
    And the output communicates that the collection was not found

  Scenario: Report a missing database without creating a file
    Given no database exists at "/tmp/mdsearch-get/missing/collections.db"
    When I run `mdsearch get Notes notes.md --database "/tmp/mdsearch-get/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-get/missing/collections.db"