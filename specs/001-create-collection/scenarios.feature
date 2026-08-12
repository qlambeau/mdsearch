Feature: Create an empty named collection

  As a developer-curator
  I want to create a named collection
  So that I can organize markdown files for later indexing and retrieval

  Scenario: Initialize the default database and create the first collection
    Given no database exists at "~/.mdsearch/collections.db"
    And "Notes" is an unused valid collection name
    When I run `mdsearch collection create Notes`
    Then the database is initialized at "~/.mdsearch/collections.db"
    And an empty collection named "Notes" exists
    And the human-readable output confirms creation of "Notes"

  Scenario: Create a collection in an explicitly selected database
    Given no database exists at "/custom/path/collections.db"
    And "Project Notes" is an unused valid collection name
    When I run `mdsearch collection create "Project Notes" --database "/custom/path/collections.db"`
    Then the database is initialized at "/custom/path/collections.db"
    And an empty collection named "Project Notes" exists in that database
    And the human-readable output confirms creation of "Project Notes"

  Scenario: Trim surrounding whitespace before storing a collection name
    Given no collection named "Notes" exists
    And the input name is "  Notes  "
    When I create the collection with the input name
    Then an empty collection named "Notes" exists
    And the human-readable output confirms creation of "Notes"

  Scenario: Preserve a collection across CLI runs
    Given an empty collection named "Notes" was created in an earlier CLI run
    When I use the same database in a later CLI run
    Then the collection remains available with the display name "Notes"

  Scenario: Reject a case-insensitive duplicate collection name
    Given an empty collection named "Notes" already exists
    When I run `mdsearch collection create notes`
    Then the operation is rejected
    And the output communicates that the collection name is already in use
    And the existing collection retains the display name "Notes"
    And no second collection is created

  Scenario Outline: Reject an invalid collection name
    Given no collection exists for the attempted name
    And the attempted name <name> is invalid because it <reason>
    When I create the collection with the attempted name
    Then the operation is rejected
    And the output communicates that the collection name is invalid
    And no collection is created

    Examples:
      | name                                      | reason                         |
      | an empty value                             | is empty                       |
      | a whitespace-only value                    | contains only whitespace       |
      | "Notes/2026"                              | contains a path separator      |
      | "Notes\\2026"                            | contains a path separator      |
      | a value containing a control character    | contains a control character   |

  Scenario: Fail without a partial collection when the database is inaccessible
    Given the selected database cannot be created or opened
    And "Notes" is an unused valid collection name
    When I run `mdsearch collection create Notes --database "/inaccessible/collections.db"`
    Then the operation fails
    And the output communicates that the database could not be accessed
    And no partial "Notes" collection exists
