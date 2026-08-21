Feature: Destroy a collection completely with no orphaned data

  As a developer-curator
  I want `mdsearch collection destroy` to remove every trace of a collection
  So that no orphaned data remains and a recreated collection never inherits stale rows

  Scenario: Destroying a fully indexed collection removes every trace
    Given a collection named "Notes" has stored files
    And "Notes" has a built lexical index, a built semantic index, and graph rows
    When I run `mdsearch collection destroy Notes`
    Then the collection "Notes" is removed
    And no rows for "Notes" remain in the files, passages, embeddings, graph, or index-state tables

  Scenario: Destroying a collection with only files removes its files
    Given a collection named "Notes" has stored files and nothing else
    When I run `mdsearch collection destroy Notes`
    Then the collection "Notes" is removed
    And no stored files remain for "Notes"

  Scenario: Destroying an unknown collection changes nothing
    Given a database exists with no collection named "Missing"
    And other collections hold indexed data
    When I run `mdsearch collection destroy Missing`
    Then the operation fails
    And the output communicates that the collection was not found
    And the database is unchanged

  Scenario: Destroying one collection leaves others fully intact
    Given collections "Notes" and "Archive" both have stored files, built indexes, and graph rows
    When I run `mdsearch collection destroy Notes`
    Then the collection "Notes" is removed
    And "Archive" still has its files, indexes, and graph rows

  Scenario: Recreating a collection after destroy surfaces no stale data
    Given "Notes" had stored files, built indexes, and graph rows
    And "Notes" was destroyed in an earlier run
    And a new collection named "Notes" was created with fresh files and built indexes
    When I search, run hybrid, and query the graph for the new "Notes"
    Then no stale passages, vectors, or graph rows from the destroyed collection appear

  Scenario: A failed destroy leaves the collection intact
    Given a collection named "Notes" has stored files, built indexes, and graph rows
    And a storage error occurs while destroying "Notes"
    When I run `mdsearch collection destroy Notes`
    Then the operation fails
    And the collection "Notes" and all of its data remain unchanged