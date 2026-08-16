Feature: Destroy a named collection

  As a developer-curator
  I want to destroy a named collection
  So that I can remove collections I no longer need

  Scenario: Destroy a collection by name
    Given a database exists with a collection named "Notes"
    When I run `mdsearch collection destroy Notes`
    Then the collection "Notes" is removed
    And the human-readable output confirms destruction of "Notes"

  Scenario: Destroy a collection case-insensitively
    Given a database exists with a collection named "Notes"
    When I run `mdsearch collection destroy notes`
    Then the collection "Notes" is removed
    And the human-readable output confirms destruction of "Notes"

  Scenario: Destroy one collection without disturbing others
    Given a database exists with collections "Notes" and "Archive"
    When I run `mdsearch collection destroy Notes`
    Then the collection "Notes" is removed
    And the collection "Archive" remains available

  Scenario: Destroy a non-existent collection in an existing database
    Given a database exists with no collection named "Missing"
    When I run `mdsearch collection destroy Missing`
    Then the operation fails
    And the output communicates that the collection was not found
    And the database is unchanged

  Scenario: Destroy in a database that does not exist
    Given no database exists at "/tmp/mdsearch-destroy/missing/collections.db"
    When I run `mdsearch collection destroy Notes --database "/tmp/mdsearch-destroy/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-destroy/missing/collections.db"

  Scenario Outline: Reject an invalid collection name
    Given no collection exists for the attempted name
    And the attempted name <name> is invalid because it <reason>
    When I destroy the collection with the attempted name
    Then the operation is rejected
    And the output communicates that the collection name is invalid
    And no collection is destroyed

    Examples:
      | name                                   | reason                       |
      | an empty value                         | is empty                     |
      | a whitespace-only value                | contains only whitespace     |
      | "Notes/2026"                          | contains a path separator    |
      | "Notes\\2026"                        | contains a path separator    |
      | a value containing a control character | contains a control character |

  Scenario: A destroyed collection no longer appears in a later listing
    Given a database exists with a collection named "Notes"
    And I destroyed "Notes" in an earlier CLI run
    When I run `mdsearch collection list`
    Then the output does not include "Notes"
