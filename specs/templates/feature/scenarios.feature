# TEMPLATE: Replace this clearly illustrative feature with the story's approved
# behavior. Keep one Feature per user story and keep steps at the behavioral
# level rather than naming UI controls or implementation methods.

Feature: Save a delivery address

  # Illustrative example only. Replace it before use.
  Scenario: Save a valid delivery address
    Given I am a returning customer
    And I have entered a valid delivery address
    When I save the address
    Then the address is available for my next checkout

  Scenario: Reject an incomplete delivery address
    Given I am a returning customer
    And the delivery address is missing a required field
    When I save the address
    Then the address is not saved
    And I am told which information is required
