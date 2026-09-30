# parent: US-023
# status: approved
# approval: Quentin approved these US-023 scenarios in chat.

Feature: Unified retrieval and final CLI surface
  Developers and coding agent harnesses use consistent commands to retrieve
  grounded content and inspect collection indexes and graphs.

  Scenario: The normal workflow uses the final command grammar
    Given a directory contains Markdown files
    When I create a collection with that source
    And I update the collection with the top-level update command
    Then I can search its indexed passages
    And I can retrieve a matching file by its reported file ID

  Scenario: Collection creation requires at least one source
    When I create a collection without source paths
    Then the command is rejected as invalid input
    And no collection is created

  Scenario: Collection configuration does not index content
    Given a collection has registered sources
    When I replace its sources and enable semantic indexing
    Then the configuration is saved
    And its stored files and indexes are unchanged until update

  Scenario: Collection listing reports sources and enabled indexes
    Given collections have different registered sources and semantic policies
    When I list collections in human or JSON format
    Then each collection's sources and enabled indexes are reported

  Scenario: Collection deletion retains original files
    Given an indexed collection has registered source files
    When I delete the collection
    Then its stored files and indexes are removed
    And its original source files remain on disk

  Scenario Outline: Obsolete commands are rejected
    When I invoke the obsolete command "<command>"
    Then the command exits with code 2
    And its argument diagnostic is written to stderr
    Examples:
      | command           |
      | collection add    |
      | collection destroy|
      | collection update |
      | embed             |
      | hybrid            |
      | index status      |
      | context           |

  Scenario: Update requires exactly one scope selector
    When I request an update without a scope or with both scope selectors
    Then the command is rejected as invalid input
    And stored collection data is unchanged

  Scenario: Structured updates retain atomic per-collection outcomes
    Given multiple collections are registered for update
    And one collection cannot rebuild a configured index
    When I update all collections in JSON format
    Then every collection's outcome is present in the structured report
    And successful collections commit their updated content and indexes
    And the failed collection retains its previous content and indexes
    And the command exits with code 1

  Scenario: Quoted and multiple-word queries are equivalent
    Given a collection has indexed passages matching "borrowing rules"
    When I search using a quoted query
    And I search using the same words as separate arguments
    Then both invocations use the same literal query
    And return the same ranked passages

  Scenario: Search defaults to lexical mode
    Given a collection has a lexical index
    When I search without selecting a mode
    Then the results use the existing lexical ranking
    And the reported mode is lexical

  Scenario: Hybrid mode preserves existing retrieval behavior
    Given a collection has the prerequisites for hybrid retrieval
    When I search in hybrid mode
    Then the results use the existing hybrid fusion and re-ranking behavior
    And the reported mode is hybrid

  Scenario: Search retains related-file enrichment
    Given matching files have graph relationships
    When I search in either mode with related-file enrichment
    Then the ranked passage results remain unchanged
    And related files are included in human or JSON output

  Scenario: Disabling re-ranking requires hybrid mode
    When I search in lexical mode with no-rerank
    Then the command is rejected as invalid input

  Scenario: Hybrid prerequisite failures include a recovery command
    Given a selected collection cannot satisfy a hybrid prerequisite
    When I search it in hybrid mode
    Then the operation fails
    And the diagnostic identifies the missing prerequisite
    And provides a command to recover
    And no model assets are downloaded

  Scenario Outline: Search limits retain their bounds
    Given a searchable collection
    When I search with limit "<limit>"
    Then the limit is "<outcome>"
    Examples:
      | limit | outcome  |
      | 1     | accepted |
      | 100   | accepted |
      | 0     | rejected |
      | 101   | rejected |

  Scenario: Search uses the default limit of ten
    Given more than ten passages match a query
    When I search without specifying a limit
    Then at most ten results are returned

  Scenario: Human search results identify collection and file
    Given passages from multiple collections match a query
    When I search in human format
    Then each result identifies its collection and file ID
    And retains its passage text, path, score, and position

  Scenario: Search JSON extends existing contracts
    Given a collection has matching passages
    When I search in either mode in JSON format
    Then all fields of the corresponding previous JSON contract remain present
    And the response identifies its mode
    And each result includes its file ID

  Scenario: Empty searches produce explicit output
    Given no indexed passage matches a query
    When I search in human format
    Then the output explicitly reports no matches
    When I search in JSON format
    Then the response has zero results and an empty results array

  Scenario Outline: Retrieval emits exact stored content
    Given an indexed file has content "<content>"
    When I retrieve it by its exact path or unique name
    And I retrieve it by its explicit file ID
    Then both invocations emit exactly the stored content
    And no newline is added to the content
    Examples:
      | content                      |
      | content without final newline|
      |                              |

  Scenario: Numeric names remain name selectors
    Given a stored file has a numeric name
    When I retrieve it with the path-or-name selector
    Then that file is selected by name
    And the selector is not interpreted as a file ID

  Scenario: Retrieval rejects conflicting selectors
    When I retrieve a file with both a path-or-name selector and an explicit ID
    Then the command is rejected as invalid input

  Scenario: Retrieval preserves scoped not-found and ambiguity errors
    Given two collections contain files with the same basename
    And one collection contains more than one file with another basename
    When I retrieve a name within a selected collection
    Then lookup is restricted to that collection
    And missing files report a file-not-found error
    And ambiguous basenames report candidate paths

  Scenario: Neighbor inspection defaults to file nodes and one hop
    Given a collection contains a file node and its graph neighbors
    When I inspect neighbors without specifying kind or depth
    Then the file node is selected
    And only one-hop neighbors are returned from the selected collection

  Scenario: Explicit neighbor options restrict traversal
    Given a collection contains file, tag, and alias nodes with typed relations
    When I select a node kind, relation, and traversal depth
    Then traversal uses that node kind
    And returns only matching relations within the requested depth

  Scenario Outline: Neighbor depth retains its bounds
    Given a collection contains an inspectable graph node
    When I inspect neighbors with depth "<depth>"
    Then the depth is "<outcome>"
    Examples:
      | depth | outcome  |
      | 1     | accepted |
      | 255   | accepted |
      | 0     | rejected |
      | 256   | rejected |

  Scenario: Graph commands require collection selection
    When I inspect graph neighbors or run a graph query without a collection
    Then the command is rejected as invalid input

  Scenario: Graph queries are bound to the selected collection
    Given Notes and Archive contain the same node key with different neighbors
    When I query that node and its neighbors with Notes selected
    Then every resolver returns data from Notes
    And no data from Archive is returned

  Scenario: GraphQL cannot override CLI collection selection
    Given Notes is selected for a graph query
    When the query supplies a collection argument to a public graph field
    Then query validation rejects the unsupported argument
    And no graph data is returned

  Scenario: Status reports readiness, enablement, models, and successful builds
    Given collections have different semantic policies and built index states
    When I request status in human or JSON format
    Then each collection's configured indexes and readiness are reported
    And selected models and last successful build times are reported
    And an unbuilt index has no successful build time

  Scenario: Status can be restricted to a collection
    Given multiple collections exist
    When I request status for one collection
    Then only that collection's status is returned

  Scenario: Freshness compares indexes with stored content
    Given an index was built from an earlier version of stored collection content
    When I request status
    Then the affected index is reported stale
    And its previous successful build time remains available

  Scenario: Pending filesystem changes do not affect status freshness
    Given a collection's indexes match its stored content
    And its registered source files changed after its last update
    When I request status
    Then its indexes remain current relative to stored content
    And status does not scan the registered sources

  Scenario: Global options work throughout the command tree
    Given an explicitly selected database
    When I place the database option at any supported command depth
    And I use short collection or search-limit options where applicable
    Then the intended database, collection, and result limit are selected

  Scenario: Diagnostics and informational commands retain their stream contract
    When I request help or version without HOME
    Then the output is written to stdout and the command exits with code 0
    When I invoke an invalid command without HOME
    Then the diagnostic is written to stderr and the command exits with code 2
    When an operation cannot complete
    Then the diagnostic is written to stderr and the command exits with code 1

  Scenario: Explicit paths permit operations without unnecessary HOME resolution
    Given every path required by an operation is explicitly provided
    When I run the operation without HOME
    Then the operation does not fail because HOME is missing

  Scenario: Help and migration documentation describe the replacement surface
    When I read command help and the old-to-new command mapping
    Then the final commands, defaults, prerequisites, and examples are documented
    And the documented workflow uses the replacement command grammar
