# parent: US-022
# status: approved

Feature: Configure semantic indexes and global models
  Developers choose which collections maintain semantic indexes and select
  shared embedding and re-ranker models without leaving collection indexes in
  a partially updated state.

  Scenario: A new collection has semantic indexing disabled by default
    Given I create a collection with registered Markdown sources
    And I do not enable semantic indexing for the collection
    When I update the collection
    Then its files, lexical index, and graph are built
    And no semantic index is built for the collection

  Scenario: Enabling semantic indexing builds it on update
    Given a collection has registered sources and lexical and graph indexes
    And semantic indexing is disabled for the collection
    When I enable semantic indexing and update the collection
    Then its semantic index is built with the selected global embedding model

  Scenario: Disabling semantic indexing removes its index on successful update
    Given semantic indexing is enabled for a collection with stored vectors
    When I disable semantic indexing and update the collection successfully
    Then the collection's vectors and semantic index state are removed

  Scenario: A failed update preserves a disabled collection's previous index state
    Given semantic indexing is enabled for a collection with stored vectors
    And I disable semantic indexing for the collection
    And one configured index cannot be rebuilt
    When I update the collection
    Then the update fails
    And the collection's previous files and indexes remain unchanged

  Scenario: Update all continues after a collection failure
    Given multiple collections are registered for update
    And one collection cannot rebuild an enabled index
    When I update all collections
    Then the other collections are still attempted
    And every collection outcome is reported
    And the command exits unsuccessfully

  Scenario: Migration preserves existing semantic enablement
    Given a legacy collection already has a semantic index
    And another legacy collection has no semantic index
    When the database is migrated
    And I update both collections
    Then the first collection's semantic index remains enabled and is refreshed
    And the second collection remains without semantic indexing

  Scenario: Model list reports support and local availability
    Given supported embedding and re-ranker models have different local availability
    When I list models
    Then supported embedding and re-ranker models are shown
    And each model's local availability is reported

  Scenario: An unavailable model cannot be selected without download permission
    Given the requested model's assets are not available locally
    When I select that model without allowing downloads
    Then the operation fails before changing model settings or indexes
    And the output explains how to download the model

  Scenario: Model selection downloads only when explicitly allowed
    Given the requested model's assets are not available locally
    When I select the model and explicitly allow downloads
    Then the model assets are fetched
    And the model setting and affected indexes are updated together

  Scenario: Changing the embedding model rebuilds every enabled semantic index
    Given multiple collections have enabled semantic indexes under embedding model "A"
    When I select embedding model "B"
    Then every enabled semantic index is rebuilt under model "B"
    And the global embedding model becomes "B"

  Scenario: A failed model change preserves the prior global model and indexes
    Given multiple collections have enabled semantic indexes under embedding model "A"
    And one collection fails while rebuilding with model "B"
    When I select embedding model "B"
    Then the operation fails
    And the global embedding model remains "A"
    And all previous semantic indexes remain unchanged

  Scenario: Selecting only a re-ranker does not rebuild semantic vectors
    Given collections have semantic indexes under the current embedding model
    When I select a re-ranker without supplying an embedding model
    Then the global re-ranker setting changes
    And the embedding model remains unchanged
    And semantic vectors are not rebuilt

  Scenario: Setting neither model is rejected
    When I try to set models without an embedding or re-ranker model
    Then the command is rejected as invalid input
    And no model setting or semantic index changes

  Scenario: The legacy embed command respects collection semantic configuration
    Given semantic indexing is disabled for a collection
    When I run the existing embed command for that collection
    Then the command fails with guidance to enable semantic indexing in collection configuration
    And the collection remains semantically disabled
