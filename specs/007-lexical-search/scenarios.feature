Feature: Search the lexical index for ranked passages

  As a developer-curator and coding-agent harness
  I want to search the lexical index and get ranked passages
  So that I can find the most relevant passages across my collections for a query

  Scenario: Search all collections returns ranked passages
    Given collections "Notes" and "Archive" have built indexes
    And both contain a passage about "borrowing"
    When I run `mdsearch search borrowing`
    Then ranked passage blocks are returned
    And the results are ordered by score, highest first
    And the output ends with a summary of the total match count

  Scenario: Restrict a search to one collection
    Given a collection named "Notes" has a built index
    When I run `mdsearch search borrowing --collection Notes`
    Then only passages from "Notes" are returned

  Scenario: Report a missing collection for --collection
    Given no collection named "Journal" exists
    When I run `mdsearch search borrowing --collection Journal`
    Then the operation fails
    And the output communicates that the collection was not found

  Scenario: Report an unbuilt index for --collection
    Given a collection named "Notes" stores files but has no built index
    When I run `mdsearch search borrowing --collection Notes`
    Then the operation fails
    And the output communicates that the index is not built

  Scenario: Skip unbuilt collections when searching all
    Given a collection named "Notes" has a built index
    And a collection named "Draft" stores files but has no built index
    When I run `mdsearch search borrowing`
    Then the search succeeds
    And passages from "Notes" are returned

  Scenario: Cap results with --limit and report the total
    Given a built collection matches more than 5 passages for "borrowing"
    When I run `mdsearch search borrowing --limit 5`
    Then at most 5 passage blocks are shown
    And the summary reports the total match count

  Scenario: Reject an out-of-range --limit
    When I run `mdsearch search borrowing --limit 200`
    Then the operation fails

  Scenario: Match passages containing all query terms
    Given a built collection has passages "rust ownership" and "rust borrow"
    When I run `mdsearch search "rust ownership"`
    Then only the passage containing all terms "rust" and "ownership" is returned

  Scenario: Treat FTS5 operator characters as literal text
    Given a built collection has a passage "a AND b" and a passage "borrowing only"
    When I run `mdsearch search "a AND"`
    Then the search succeeds
    And only passages containing the literal terms "a" and "AND" are returned

  Scenario: Fail on an empty query
    When I run `mdsearch search ""`
    Then the operation fails

  Scenario: Produce empty output when nothing matches
    Given a built collection exists
    When I run `mdsearch search zzznotaword`
    Then the output is empty

  Scenario: Report a missing database without creating a file
    Given no database exists at "/tmp/mdsearch-search/missing/collections.db"
    When I run `mdsearch search borrowing --database "/tmp/mdsearch-search/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-search/missing/collections.db"