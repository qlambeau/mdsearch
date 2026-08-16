Feature: List all collections

  As a developer-curator
  I want to list all collections in a database
  So that I can see which collections exist before adding files, searching, or destroying one

  Scenario: List collections in case-insensitive alphabetical order
    Given a database exists with collections "Notes" and "Archive"
    When I run `mdsearch collection list`
    Then the output lists "Archive" then "Notes", one per line

  Scenario: List collections ignoring letter case in the sort order
    Given a database exists with collections "banana", "Apple", and "cherry"
    When I run `mdsearch collection list`
    Then the output lists "Apple", "banana", and "cherry", one per line

  Scenario: List an existing database with no collections
    Given a database exists with no collections
    When I run `mdsearch collection list`
    Then the output is empty

  Scenario: List a database that does not exist
    Given no database exists at "/tmp/mdsearch-list/missing/collections.db"
    When I run `mdsearch collection list --database "/tmp/mdsearch-list/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-list/missing/collections.db"

  Scenario: List a database that cannot be opened
    Given the database exists but cannot be opened
    When I run `mdsearch collection list`
    Then the operation fails
    And the output communicates that the database could not be accessed

  Scenario: List a collection created in an earlier CLI run
    Given a collection named "Notes" was created in an earlier CLI run
    When I run `mdsearch collection list` in a later CLI run
    Then the output includes "Notes"
