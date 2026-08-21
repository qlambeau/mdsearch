Feature: Model downloads live under .mdsearch with reliable availability detection

  As a developer-curator and coding-agent harness
  I want model assets downloaded with `--download` to be stored under the `.mdsearch` data directory and availability detected by a completion marker
  So that everything the tool persists lives in one place and `embed`/`hybrid` never tell me to download a model I already have

  Scenario: A download stores the model under ~/.mdsearch/models with no environment overrides
    Given no HF_HOME or FASTEMBED_CACHE_DIR environment variable is set
    And a collection named "Notes" has files and a built lexical index
    When I run `mdsearch embed --download` from any working directory
    Then the embed succeeds
    And the model assets for the default model are stored under the home directory `.mdsearch/models`
    And a completion marker exists for the default model

  Scenario: A previously downloaded model is found from a different working directory
    Given the default model assets were downloaded into the home directory `.mdsearch/models` with a completion marker
    When I run `mdsearch embed` without `--download` from a different working directory
    Then the embed succeeds
    And the output does not suggest passing `--download`

  Scenario: HF_HOME overrides the default model cache location
    Given HF_HOME is set to a custom cache directory
    And a collection named "Notes" has files and a built lexical index
    When I run `mdsearch embed --download`
    Then the model assets are stored under the HF_HOME cache directory
    And a completion marker exists for the default model

  Scenario: An interrupted download does not count as downloaded
    Given a previous `--download` run left partial model assets without a completion marker
    When I run `mdsearch embed` without `--download`
    Then the operation fails
    And the output suggests passing `--download`

  Scenario: Partial assets without a completion marker are not considered downloaded
    Given model files exist under the resolved cache directory but no completion marker exists for the model
    When I run `mdsearch embed` without `--download`
    Then the operation fails
    And the output suggests passing `--download`

  Scenario: The reranker follows the same cache location and marker rules
    Given no HF_HOME or FASTEMBED_CACHE_DIR environment variable is set
    And a collection named "Notes" has files and a built lexical index
    When I run `mdsearch hybrid borrowing --reranker bge-reranker-base --download`
    Then the reranker assets are stored under the home directory `.mdsearch/models`
    And a completion marker exists for the reranker model
    When I run `mdsearch hybrid borrowing --reranker bge-reranker-base` without `--download` from a different working directory
    Then the hybrid search succeeds
    And the output does not suggest passing `--download`

  Scenario: A --database override does not change the model location
    Given no HF_HOME or FASTEMBED_CACHE_DIR environment variable is set
    And a collection named "Notes" has files and a built lexical index
    When I run `mdsearch embed --download --database /elsewhere/collections.db`
    Then the embed succeeds
    And the model assets are stored under the home directory `.mdsearch/models`
    And no model assets are stored next to the database file

  Scenario Outline: The model cache resolution order is honored
    Given "<env>" is set to a custom cache directory
    And a collection named "Notes" has files and a built lexical index
    When I run `mdsearch embed --download`
    Then the model assets are stored under the environment cache directory
    And a completion marker exists for the default model

    Examples:
      | env                    |
      | HF_HOME                |
      | FASTEMBED_CACHE_DIR    |