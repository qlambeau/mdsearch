# parent: US-020
# status: approved

Feature: CLI help, diagnostics, and global options
  Developer-curators and harnesses can discover commands and interpret results.

  Scenario Outline: Informational commands work without HOME
    Given HOME is absent from the subprocess environment
    When I run mdsearch with "<arguments>"
    Then the subprocess exits with code 0
    And informational output is written to stdout
    And stderr is empty

    Examples:
      | arguments               |
      | --help                  |
      | --version               |
      | collection --help       |
      | collection add --help   |
      | graph neighbors --help  |

  Scenario: Argument errors use stderr and exit code 2
    Given HOME is absent from the subprocess environment
    When I run mdsearch with an unknown option
    Then the subprocess exits with code 2
    And stdout is empty
    And stderr describes the invalid argument

  Scenario: Operational failures use stderr and exit code 1
    Given an explicit database path that does not exist
    When I list collections in that database
    Then the subprocess exits with code 1
    And stdout is empty
    And stderr describes the missing database

  Scenario Outline: Database selection is global
    Given HOME is absent from the subprocess environment
    And an explicit writable database path
    When I create a collection placing the database option "<placement>"
    Then the subprocess exits with code 0
    And the collection exists in the explicitly selected database

    Examples:
      | placement                    |
      | before collection            |
      | between collection and create |
      | after create arguments       |

  Scenario: A missing HOME is diagnosed only when a default path needs it
    Given HOME is absent from the subprocess environment
    When I create a collection without an explicit database path
    Then the subprocess exits with code 1
    And stdout is empty
    And stderr describes the unavailable home directory

  Scenario: Collection and limit short switches select search results
    Given Notes is a searchable collection with more than one matching passage
    When I search for rust with "-c Notes -n 1"
    Then only results from Notes are returned
    And at most one result is returned

  Scenario Outline: Search limit boundaries remain enforced
    Given a searchable collection
    When I search with limit "<limit>"
    Then the subprocess exits with code <exit_code>

    Examples:
      | limit | exit_code |
      | 1     | 0         |
      | 100   | 0         |
      | 0     | 2         |
      | 101   | 2         |

  Scenario: Search uses a default limit of ten
    Given a searchable collection with more than ten matching passages
    When I search without a limit switch
    Then at most ten results are returned

  Scenario Outline: Command help describes use and examples
    When I request help for "<command>"
    Then help describes the command purpose and effects
    And help explains applicable prerequisites and defaults
    And help contains a command example

    Examples:
      | command            |
      | mdsearch           |
      | collection         |
      | collection create  |
      | collection list    |
      | collection destroy |
      | collection add     |
      | collection update  |
      | index              |
      | index status       |
      | search             |
      | get                |
      | embed              |
      | hybrid             |
      | graph              |
      | graph neighbors    |
      | context            |

  Scenario Outline: The ingestion switch is named for unreadable-file handling
    Given an existing collection and a readable markdown file
    And a second supplied file path cannot be read
    When I run "<command>" with both paths and --skip-unreadable
    Then the readable file is processed
    And the unreadable path is reported as skipped

    Examples:
      | command           |
      | collection add    |
      | collection update |

  Scenario: The old force switch is rejected
    When I invoke collection ingestion with --force
    Then the subprocess exits with code 2
    And stderr describes the invalid argument

  Scenario: Ingestion documentation matches the switch behavior
    When I read command help and the README ingestion reference
    Then --skip-unreadable is described as skipping unreadable files
    And the switch is not described as forcing reprocessing
