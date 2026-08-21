Feature: Unify literal free-text query semantics across lexical and hybrid search

  As a developer-curator and coding-agent harness
  I want `search` and `hybrid` to treat the same query string as literal free text
  So that identical queries return consistent results on both commands

  Scenario: The same query string returns the same passages on both commands
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And "Notes" contains a passage "borrowing rust" and a passage "borrowing only"
    And the re-ranker assets are cached locally
    When I run `mdsearch search "borrowing rust"`
    Then the passage "borrowing rust" is returned
    And the passage "borrowing only" is not returned
    When I run `mdsearch hybrid "borrowing rust"`
    Then the passage "borrowing rust" is returned
    And the passage "borrowing only" is not returned

  Scenario Outline: FTS5 operator characters match literally on both commands
    Given a collection named "Notes" has a built lexical index and a built semantic index
    And "Notes" contains a passage "<matching>" and a passage "unrelated notes"
    And the re-ranker assets are cached locally
    When I run `mdsearch <command> "<query>"`
    Then the command succeeds
    And the passage "<matching>" is returned
    And the passage "unrelated notes" is not returned

    Examples:
      | command | query             | matching                 |
      | search  | a AND b           | a AND b semantics        |
      | hybrid  | a AND b           | a AND b semantics        |
      | search  | rust OR ownership | rust OR ownership rules  |
      | hybrid  | rust OR ownership | rust OR ownership rules  |
      | search  | borrowing*        | borrowing is easy        |
      | hybrid  | borrowing*        | borrowing is easy        |

  Scenario: An empty query fails on both commands
    When I run `mdsearch search ""`
    Then the operation fails
    When I run `mdsearch hybrid ""`
    Then the operation fails

  Scenario: A whitespace-only query fails on both commands
    When I run `mdsearch search "   "`
    Then the operation fails
    When I run `mdsearch hybrid "   "`
    Then the operation fails

  Scenario: Re-run the golden evaluation after the change
    When I run `cargo xtask eval`
    Then the evaluation passes against the ADR-004 targets (Recall@5 >= 0.85, MRR@5 >= 0.70, NDCG@5 >= 0.75)
    And the recorded baseline is updated if the scores shifted