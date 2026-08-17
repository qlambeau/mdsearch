Feature: Show passage positions and machine-readable JSON for search

  As a developer-curator and coding-agent harness
  I want search results to show where each matching passage sits in its file and to get machine-readable JSON
  So that I can locate passages precisely and feed structured results to a harness

  Scenario: Show the passage line range in the human output
    Given a built collection has "notes.md" with the passage "borrowing rules" on lines 12 to 16
    When I run `mdsearch search borrowing`
    Then a result header shows "notes.md:12-16"
    And the passage text follows on the next line

  Scenario: Emit a structured JSON object for a search
    Given a built collection has a passage matching "borrowing"
    When I run `mdsearch search borrowing --json`
    Then the output is exactly one JSON object
    And it contains the query, collection scope, limit, and total
    And its results array entries have collection, path, kind, text, score, and position

  Scenario: Emit valid JSON with empty results when nothing matches
    Given a built collection exists
    When I run `mdsearch search zzz --json`
    Then the output is a valid JSON object
    And its results array is empty and total is 0

  Scenario: Produce empty human output when nothing matches
    Given a built collection exists
    When I run `mdsearch search zzz`
    Then the output is empty

  Scenario: Fail clearly on a malformed query in JSON mode
    Given a built collection exists
    When I run `mdsearch search "a AND" --json`
    Then the operation fails
    And the output communicates a query error
    And no JSON is emitted

  Scenario: Fail on an empty query in JSON mode
    Given a built collection exists
    When I run `mdsearch search "" --json`
    Then the operation fails

  Scenario: Honor --limit in JSON and report the total
    Given a built collection matches more than 5 passages for "borrowing"
    When I run `mdsearch search borrowing --json --limit 5`
    Then the results array has at most 5 entries
    And the total reports the full match count