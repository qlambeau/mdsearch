Feature: Wikilink graph extraction

  As a developer-curator whose vault uses Obsidian-style wikilinks
  I want `mdsearch update` to extract `[[note]]`, `[[note|label]]`, and `[[path/note#heading]]` as LINKS_TO graph edges
  So that wikilink-based vaults produce connected graphs and `--related`/`graph neighbors` surface real relationships

  Scenario: A plain wikilink creates a LINKS_TO edge
    Given a vault with files "a.md" containing `[[borrowing]]` and "borrowing.md"
    When I run `mdsearch collection update` on the vault
    Then the graph contains a LINKS_TO edge from "a.md" to "borrowing.md"

  Scenario: A piped wikilink targets the file, ignoring the label
    Given a vault with files "a.md" containing `[[notes/borrowing|Borrowing Rules]]` and "notes/borrowing.md"
    When I run `mdsearch collection update` on the vault
    Then the graph contains a LINKS_TO edge from "a.md" to "notes/borrowing.md"
    And no alias node is created for the label

  Scenario: A header fragment is stripped before resolution
    Given a vault with files "a.md" containing `[[borrowing#Lifetimes]]` and "borrowing.md"
    When I run `mdsearch collection update` on the vault
    Then the graph contains a LINKS_TO edge from "a.md" to "borrowing.md"

  Scenario: A bare self-anchor produces no edge
    Given a vault with a file "a.md" containing `[[#Overview]]`
    When I run `mdsearch collection update` on the vault
    Then the graph contains no edge from "a.md" to itself

  Scenario: Wikilink resolution is case-insensitive
    Given a vault with files "a.md" containing `[[Note]]` and "note.md"
    When I run `mdsearch collection update` on the vault
    Then the graph contains a LINKS_TO edge from "a.md" to "note.md"

  Scenario: An unresolved wikilink produces no edge
    Given a vault with a file "a.md" containing `[[missing]]`
    When I run `mdsearch collection update` on the vault
    Then the graph contains no LINKS_TO edge from "a.md"

  Scenario: An ambiguous case-only match is skipped
    Given a vault with files "a.md" containing `[[Note]]`, "Note.md", and "note.md"
    When I run `mdsearch collection update` on the vault
    Then the graph contains no LINKS_TO edge from "a.md" to "Note.md" or "note.md"

  Scenario: A self-link produces no edge
    Given a vault with a file "note.md" containing `[[note]]`
    When I run `mdsearch collection update` on the vault
    Then the graph contains no edge from "note.md" to itself

  Scenario: Markdown links and wikilinks both produce edges
    Given a vault with files "a.md" containing both `[label](target.md)` and `[[other]]`, "target.md", and "other.md"
    When I run `mdsearch collection update` on the vault
    Then the graph contains a LINKS_TO edge from "a.md" to "target.md"
    And the graph contains a LINKS_TO edge from "a.md" to "other.md"

  Scenario: A wikilink resolves to a file through a path
    Given a vault with files "a.md" containing `[[subdir/note]]` and "subdir/note.md"
    When I run `mdsearch collection update` on the vault
    Then the graph contains a LINKS_TO edge from "a.md" to "subdir/note.md"