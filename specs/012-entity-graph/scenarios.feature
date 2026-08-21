Feature: Deterministic entity graph build and internal query layer

  As a developer-curator and coding-agent harness
  I want `mdsearch update` to build an entity graph of files, tags, and aliases with typed edges from frontmatter and inline links
  So that related concepts can be surfaced with zero LLM or network cost

  Scenario: Update builds file and tag nodes with LINKS_TO and TAGGED_WITH edges
    Given a collection named "Notes" stores two files "a.md" and "b.md"
    And "a.md" contains a relative link to "b.md"
    And both files have frontmatter tags "rust"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has file nodes "a.md" and "b.md"
    And the graph for "Notes" has a tag node "rust"
    And the graph for "Notes" has a LINKS_TO edge from "a.md" to "b.md"
    And the graph for "Notes" has TAGGED_WITH edges from "a.md" and "b.md" to "rust"

  Scenario: Update creates alias nodes and ALIAS_OF edges
    Given a collection named "Notes" stores a file "a.md" with frontmatter aliases "mt" and "my"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has alias nodes "mt" and "my"
    And the graph for "Notes" has ALIAS_OF edges from "a.md" to "mt" and "my"

  Scenario: Update creates RELATED_TO and HAS_SOURCE edges
    Given a collection named "Notes" stores files "a.md", "b.md", and "c.md"
    And "a.md" has frontmatter related "b.md"
    And "a.md" has frontmatter sources "c.md"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has a RELATED_TO edge from "a.md" to "b.md"
    And the graph for "Notes" has a HAS_SOURCE edge from "a.md" to "c.md"

  Scenario: Skip unresolved related and sources references
    Given a collection named "Notes" stores a file "a.md" with frontmatter related "missing.md"
    And "missing.md" does not exist in "Notes"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has no edge for the unresolved reference "missing.md"

  Scenario: Rebuild drops nodes and edges from deleted files
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    And the graph for "Notes" has a LINKS_TO edge from "a.md" to "b.md"
    When "b.md" is deleted from the filesystem
    And I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has no file node "b.md"
    And the graph for "Notes" has no LINKS_TO edge from "a.md" to "b.md"

  Scenario: Rebuild on unchanged files produces the same graph
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    When I run `mdsearch update`
    And I run `mdsearch update` again
    Then the graph for "Notes" has exactly one file node "a.md"
    And the graph for "Notes" has exactly one LINKS_TO edge from "a.md" to "b.md"

  Scenario: An empty collection builds an empty graph
    Given a collection named "Empty" stores no files
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Empty" is empty

  Scenario: Alias and tag nodes with the same name remain distinct
    Given a collection named "Notes" stores a file "a.md" with frontmatter tags "mt" and aliases "mt"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has a tag node "mt"
    And the graph for "Notes" has an alias node "mt"
    And the tag node "mt" and the alias node "mt" are distinct rows

  Scenario: Inspect a node's neighbors with the debug CLI
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    And the graph for "Notes" has a LINKS_TO edge from "a.md" to "b.md"
    When I run `mdsearch graph neighbors a.md`
    Then the command succeeds
    And the output lists "b.md" as a neighbor of "a.md"
    And the output reports the relation type "LINKS_TO" and depth "1"

  Scenario: Query layer expands neighbors along a relation filter
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" links to "b.md"
    And the graph for "Notes" has a LINKS_TO edge from "a.md" to "b.md"
    When I query the internal graph layer for neighbors of "a.md" filtered by relation "LINKS_TO"
    Then the query returns "b.md"

  Scenario: Query layer traversal stops at the hop limit
    Given a collection named "Notes" stores files "a.md", "b.md", and "c.md"
    And "a.md" links to "b.md"
    And "b.md" links to "c.md"
    When I query the internal graph layer for a 1-hop traversal from "a.md"
    Then the traversal returns "b.md"
    And the traversal does not return "c.md"

  Scenario: Update builds the graph with no network or LLM dependency
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And no network or LLM service is available
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has file nodes "a.md" and "b.md"

  Scenario: Update extracts wikilinks as LINKS_TO edges
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" contains a wikilink "[[b]]" to "b.md"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has a LINKS_TO edge from "a.md" to "b.md"

  Scenario: Update strips wikilink fragments and ignores piped labels
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" contains a wikilink "[[b#Lifetimes|Borrowing]]" to "b.md"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has a LINKS_TO edge from "a.md" to "b.md"
    And no alias node is created for "Borrowing"

  Scenario: Update resolves wikilinks case-insensitively and skips ambiguity
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" contains a wikilink "[[B]]" to "b.md"
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has a LINKS_TO edge from "a.md" to "b.md"
    Given a collection named "Notes" additionally stores a file "B.md"
    And "a.md" contains a wikilink "[[B]]"
    When I run `mdsearch update`
    Then the graph for "Notes" has no edge for the ambiguous wikilink "[[B]]"

  Scenario: Update skips unresolved wikilinks and self-links
    Given a collection named "Notes" stores files "a.md" and "b.md"
    And "a.md" contains a wikilink "[[missing]]" to an unknown file
    And "a.md" contains a wikilink "[[a]]" to itself
    When I run `mdsearch update`
    Then the update succeeds
    And the graph for "Notes" has no edge for "[[missing]]"
    And the graph for "Notes" has no edge from "a.md" to itself