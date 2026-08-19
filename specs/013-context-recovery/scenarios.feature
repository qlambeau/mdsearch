Feature: Recover context from the entity graph

  As a developer-curator and coding-agent harness
  I want to recover file-to-file related context from the entity graph via --related on search/hybrid and the mdsearch context command
  So that I can fill an LLM context window with grounded, related knowledge at zero LLM/network cost

  Scenario: --related lists file-to-file related links in human output
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    And both files have frontmatter tags "rust"
    And I run `mdsearch search rust --related`
    Then the output under the "a.md" result lists "related: b.md (LINKS_TO)"
    And the output does not list the tag "rust" as related

  Scenario: --related omits tag and alias nodes
    Given a collection named "Notes" stores a file "a.md"
    And "a.md" has frontmatter tags "rust" and aliases "mt"
    And "a.md" has no file-to-file links
    When I run `mdsearch search rust --related`
    Then the output for the "a.md" result has no related line
    And the command succeeds

  Scenario: --related adds a related field to JSON output
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    When I run `mdsearch search rust --related --json`
    Then the JSON result for "a.md" includes a related entry for "b.md" with relation "LINKS_TO"

  Scenario: --related works on hybrid search
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    When I run `mdsearch hybrid rust --related`
    Then the output under the "a.md" result lists "related: b.md (LINKS_TO)"

  Scenario: --related does not change ranked results
    Given a collection named "Notes" stores files "a.md" and "b.md"
    When I run `mdsearch search rust` and `mdsearch search rust --related`
    Then the ranked result paths are identical in both runs

  Scenario: mdsearch context returns neighbors as JSON
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    When I run `mdsearch context '{ neighbors(collection: "Notes", kind: "file", key: "a.md", maxHops: 2) { key relation depth } }' --collection Notes`
    Then the command succeeds
    And the JSON output includes "b.md" with relation "LINKS_TO" and depth "1"

  Scenario: mdsearch context supports node lookup
    Given a collection named "Notes" stores a file "a.md"
    When I run `mdsearch context '{ node(collection: "Notes", kind: "file", key: "a.md") { key } }' --collection Notes`
    Then the command succeeds
    And the JSON output includes the node key "a.md"

  Scenario: mdsearch context requires a collection
    Given a collection named "Notes" exists
    When I run `mdsearch context '{ node(collection: "Notes", kind: "file", key: "a.md") { key } }'` without `--collection`
    Then the command fails with a clear error

  Scenario: mdsearch context reports an unknown node
    Given a collection named "Notes" stores a file "a.md"
    When I run `mdsearch context '{ node(collection: "Notes", kind: "file", key: "zzz.md") { key } }' --collection Notes`
    Then the command reports the node is not found

  Scenario: mdsearch context rejects a malformed query
    Given a collection named "Notes" exists
    When I run `mdsearch context 'not graphql' --collection Notes`
    Then the command reports a query error

  Scenario: Context recovery reports a missing database without creating one
    Given no database exists at the selected path
    When I run `mdsearch context '{ node(collection: "Notes", kind: "file", key: "a.md") { key } }' --collection Notes --database MISSING`
    Then the command fails
    And no database file is created
