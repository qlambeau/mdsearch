Feature: Build the semantic index with the embed command

  As a developer-curator and coding-agent harness
  I want `mdsearch embed` to build and keep current a per-passage semantic (vector) index
  So that conceptual, non-keyword queries can later be matched against my markdown passages

  Scenario: Embed builds the semantic index from the lexical passages
    Given a collection named "Notes" stores "a.md" with 3 body paragraphs, a title, and tags
    And the lexical index for "Notes" was built with 5 passages
    And the model assets are cached locally
    When I run `mdsearch embed`
    Then the embed succeeds
    And the summary reports "Notes" embedded
    And the summary reports 5 passages embedded for "Notes"
    And the summary reports the model used

  Scenario: Re-running embed with unchanged files reports already current
    Given a collection named "Notes" stores "a.md"
    And the lexical index for "Notes" was built
    And the semantic index for "Notes" was embedded
    And the stored files and model are unchanged
    When I run `mdsearch embed`
    Then the embed succeeds
    And the summary reports "Notes" as already current

  Scenario: Embed rebuilds after the file set changed
    Given a collection named "Notes" stores "a.md" with 3 body paragraphs
    And the lexical index for "Notes" was built with 3 passages
    And the semantic index for "Notes" was embedded
    And "a.md" on disk now has 5 body paragraphs
    And `mdsearch collection update Notes a.md` was run
    When I run `mdsearch embed`
    Then the embed succeeds
    And the summary reports 5 passages embedded for "Notes"

  Scenario: A --model switch rebuilds every embedded collection under the new model
    Given collections "Notes" and "Archive" have built lexical indexes and embedded semantic indexes under model "alpha"
    And the model assets for "beta" are cached locally
    When I run `mdsearch embed --model beta`
    Then the embed succeeds
    And the summary reports "Notes" embedded under model "beta"
    And the summary reports "Archive" embedded under model "beta"

  Scenario: A --model switch rebuilds embedded collections even under a narrow scope
    Given collections "Notes" and "Archive" have embedded semantic indexes under model "alpha"
    And the model assets for "beta" are cached locally
    When I run `mdsearch embed --model beta --collection Notes`
    Then the embed succeeds
    And the summary reports "Notes" embedded under model "beta"
    And the summary reports "Archive" embedded under model "beta"

  Scenario: A 1024-dimension model embeds without a dimension error
    Given a collection named "Notes" has files and a built lexical index
    And the model assets for "bge-large-en-v1.5" are cached locally
    When I run `mdsearch embed --model bge-large-en-v1.5`
    Then the embed succeeds
    And the summary reports "Notes" embedded under "bge-large-en-v1.5"
    And no dimension error is reported

  Scenario: Rebuilding under a different-dimension model recreates the vector table
    Given a collection named "Notes" has files and a built lexical index
    And "Notes" was embedded under the 1024-dimension model "bge-large-en-v1.5"
    And the default model assets are cached locally
    When I run `mdsearch embed`
    Then the embed succeeds
    And "Notes" is re-embedded under the default model
    And the semantic index for "Notes" is rebuilt at the default model's dimension

  Scenario: Unsupported model fails before any collection work
    Given a collection named "Notes" has files and a built lexical index
    When I run `mdsearch embed --model bogus`
    Then the operation fails
    And the output communicates that the model is not supported
    And no semantic index is built for "Notes"

  Scenario: Missing local model fails before any collection work without --download
    Given a collection named "Notes" has files and a built lexical index
    And the model "beta" is not cached locally
    When I run `mdsearch embed --model beta`
    Then the operation fails
    And the output names the model "beta"
    And the output suggests `--download`
    And no semantic index is built for "Notes"

  Scenario: --download fetches the model and embeds in the same run
    Given a collection named "Notes" has files and a built lexical index
    And the model is not cached locally
    When I run `mdsearch embed --download`
    Then the embed succeeds
    And the model assets are fetched
    And the summary reports "Notes" embedded

  Scenario: A failed --download modifies no collection
    Given a collection named "Notes" has files and a built lexical index
    And the model is not cached locally
    And fetching the model assets fails
    When I run `mdsearch embed --download`
    Then the operation fails
    And the output communicates the fetch failure
    And no semantic index is built for "Notes"

  Scenario: Unbuilt lexical index is skipped in all-collections mode
    Given collections "Notes" and "Archive" each store files
    And the lexical index for "Notes" was built
    And the lexical index for "Archive" was never built
    When I run `mdsearch embed`
    Then the embed succeeds
    And the summary reports "Notes" embedded
    And the summary reports "Archive" skipped

  Scenario: Unbuilt lexical index fails when explicitly targeted
    Given a collection named "Archive" stores files
    And the lexical index for "Archive" was never built
    When I run `mdsearch embed --collection Archive`
    Then the operation fails
    And the output communicates that the index is not built

  Scenario: A collection with no stored files is skipped and reported
    Given a collection named "Empty" has no stored files
    When I run `mdsearch embed`
    Then the embed succeeds
    And the summary reports "Empty" skipped

  Scenario: A collection with no stored files is skipped even when targeted
    Given a collection named "Empty" has no stored files
    When I run `mdsearch embed --collection Empty`
    Then the embed succeeds
    And the summary reports "Empty" skipped

  Scenario: Unknown collection fails when explicitly targeted
    Given no collection named "Journal" exists
    When I run `mdsearch embed --collection Journal`
    Then the operation fails
    And the output communicates that the collection was not found

  Scenario: A per-collection failure is reported and processing continues
    Given collections "Notes" and "Archive" have built lexical indexes
    And embedding "Archive" fails
    When I run `mdsearch embed`
    Then the embed succeeds for "Notes"
    And the summary reports "Archive" failed
    And the exit status reflects the failure

  Scenario: Embed failure leaves the previous semantic index intact
    Given a collection named "Notes" has a built lexical index
    And the semantic index for "Notes" was embedded
    And a storage error occurs while rebuilding the semantic index
    When I run `mdsearch embed`
    Then the operation fails
    And the previous semantic index for "Notes" is unchanged

  Scenario: Report a missing database without creating a file
    Given no database exists at "/tmp/mdsearch-embed/missing/collections.db"
    When I run `mdsearch embed --database "/tmp/mdsearch-embed/missing/collections.db"`
    Then the operation fails
    And the output communicates that the database does not exist
    And no database file is created at "/tmp/mdsearch-embed/missing/collections.db"
