Feature: Embed collections at the selected model's embedding dimension

  As a developer-curator and coding-agent harness
  I want `mdsearch embed` to index each collection at its selected model's embedding dimension
  So that every supported model, including 1024-dimension models, embeds and searches without a dimension error

  Scenario: A 1024-dimension model embeds and searches successfully
    Given a collection named "Notes" has files and a built lexical index
    And the model assets for "bge-large-en-v1.5" are cached locally
    When I run `mdsearch embed --model bge-large-en-v1.5`
    Then the embed succeeds
    And the summary reports "Notes" embedded under "bge-large-en-v1.5"
    And a subsequent `mdsearch hybrid borrowing` succeeds against "Notes"

  Scenario: Rebuilding with a different-dimension model recreates the index
    Given a collection named "Notes" has files and a built lexical index
    And "Notes" was embedded under the 1024-dimension model "bge-large-en-v1.5"
    And the default model assets are cached locally
    When I run `mdsearch embed`
    Then the embed succeeds
    And "Notes" is re-embedded under the default model
    And the semantic index for "Notes" is rebuilt at the default model's dimension

  Scenario: Status reports the recorded embedding model and dimension
    Given a collection named "Notes" has files and a built lexical index
    And "Notes" was embedded under the model "bge-large-en-v1.5"
    When I run `mdsearch index status`
    Then the output reports the embedding model "bge-large-en-v1.5" for "Notes"
    And the output reports the embedding dimension for "Notes"

  Scenario: A dimension mismatch on the semantic leg reports a clear error
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And the stored vectors for "Notes" disagree with the recorded dimension
    When I run `mdsearch hybrid borrowing`
    Then the operation fails
    And the output communicates a dimension mismatch
    And no partial results are returned

  Scenario: A legacy semantic index without a recorded dimension keeps working
    Given a collection named "Notes" has a built lexical index
    And "Notes" has a semantic index with no recorded model or dimension
    When I run `mdsearch hybrid borrowing`
    Then the hybrid search succeeds
    When I run `mdsearch embed`
    Then the embed succeeds
    And the semantic index for "Notes" records the model and dimension

  Scenario Outline: Every supported model embeds at its own dimension
    Given a collection named "Notes" has files and a built lexical index
    And the model assets for "<model>" are cached locally
    When I run `mdsearch embed --model <model>`
    Then the embed succeeds
    And no dimension error is reported

    Examples:
      | model                    |
      | all-MiniLM-L6-v2         |
      | bge-small-en-v1.5        |
      | bge-base-en-v1.5         |
      | bge-large-en-v1.5        |
      | multilingual-e5-small    |
      | multilingual-e5-base     |
      | multilingual-e5-large    |