---
layout: default
title: SPE BDD Scenarios
---

# Translated G1.F1.cart.feature

Here is a translated Gherkin`.feature` file. Not all scenarios are translated but the majority is to show the fetures of
`choreo` and that it has
parity.

### Gherkin

```gherkin
@parallel
Scenario: G1-F1-S02 Test private cart with only goods that has regular sales price
Given that cart is valid for "G1-F1-S02" to execute "step1"
#Step 1 : Request without provide single view flag
When request is sent to SPE for "G1-F1-S02"
Then response of scenario "G1-F1-S02" must have 200 statuscode
And response of step "step1" must match with the expected result
#Step 2 : Request having provide single view flag as true
When request sent to SPE for step "step2"
Then response of scenario "G1-F1-S02" must have 200 statuscode
And response of step "step2" must match with the expected result
#Step 3 : Request having provide single view flag as false
When request sent to SPE for step "step3"
Then response of scenario "G1-F1-S02" must have 200 statuscode
And response of step "step3" must match with the expected result
```

### `choreo` DSL as a `.chor` file

```choreo
feature "G1-F1 - Cart Scenarios"

actors {
    Web
    FileSystem
    System
}

settings {
    stop_on_failure = true
}

# Define the base URL and any other common variables here.
var BASE_URL = "<redacted>/sales-price"
var LEGAL_COMPANY = "<redacted>"
var OK_SCENARIOS = ["S52","S53","S54","S55","S62","S64","S65"]
var BAD_TS_SCENARIOS = ["S56","S60","S61","S63","S68","S70"]


background {
    System uuid as UUID
    Web set_header "User-Agent" "choreo-test-runner/1.0"
    Web set_header "Content-Type" "application/json"
    Web set_header "x-legal-company" "${LEGAL_COMPANY}"
    #Web set_header "x-flow-id" "${UUID}" # This should be generated
}

# This scenario corresponds to the first one in the .feature file.
parallel scenario "G1-F1-S01 Health check of SPE APIs" {

    test HealthCheckReturns200 "it returns a successful status code" {
        given:
            # Given conditions are set in the background block
            Test can_start
            Web set_header "x-flow-id" "G1-F1-S01-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S01/request.json" as REQUEST
        when:
            # The Java code reads a request.json file. We can embed that content here.
            # This makes the test self-contained.
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            # Validate basic structure
            Web response_status_is 200
            Web json_body has_path "/clientSystem"
            Web json_body has_path "/items"
            Web json_body has_path "/orderSummary"

            # Validate specific values
            Web json_path at "/salesChannel" equals "Web"
            Web json_path at "/currencyCode" equals "SEK"

            # Validate array structure
            Web json_response at "/items" is_an_array
            Web json_response at "/items" has_size 1

            # Validate item details
            Web json_path at "/items/0/itemNo" equals "70373529"
            Web json_path at "/items/0/quantity" equals 1

            # Validate pricing structure exists
            Web json_body has_path "/items/0/unitPrice"
            Web json_body has_path "/orderSummary/summary"
            Web json_response at "/orderSummary/summary" is_an_array

    }
}

# This scenario corresponds to the second, multi-step one in the .feature file.
# In choreo, we break this down into a chain of dependent tests for clarity.
parallel scenario "G1-F1-S02 Test private cart with regular sales price" {

    # The first step from the Java code
    test RegularPriceWithoutSingleView "it gets the regular price without the single view flag" {
        given:
            Test can_start
            Web set_header "x-flow-id" "G1-F1-S02-step1-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S02/step1/request.json" as REQUEST
        when:
            # This would be the content of the request.json for G1-F1-S02 step1
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            # Validate specific values
            Web json_path at "/clientSystem" as CLIENT
            Web json_path at "/salesChannel" equals "Web"
    }

    # The second step, which depends on the first.
    test RegularPriceWithSingleView "it gets the regular price with the single view flag" {
        given:
            Test has_succeeded RegularPriceWithoutSingleView
            System log "${CLIENT}"
            Web set_header "x-flow-id" "G1-F1-S02-step2-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S02/step2/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S02/step2/response.json" as RESPONSE
        when:
            # This would be the content of the request.json for G1-F1-S02 step2
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            # Validate specific values
            Web json_path at "/salesChannel" equals "Web"
            # Compare and validate with expected json
            Web response_body_equals_json "${RESPONSE}"
    }

    # The third step, which depends on the second.
    test RegularPriceWithSingleViewFalse "it gets the regular price with the single view flag set to false" {
        given:
            Test has_succeeded RegularPriceWithSingleView
            Web set_header "x-flow-id" "G1-F1-S02-step3-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S02/step3/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S02/step3/response.json" as RESPONSE
        when:
            # This would be the content of the request.json for G1-F1-S02 step3
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            # Validate specific values
            Web json_path at "/salesChannel" equals "Web"
            # Compare and validate with expected json
            Web response_body_equals_json "${RESPONSE}"
    }

}

# This scenario corresponds to the third, multi-step one in the .feature file (G1-F1-S03)
parallel scenario "G1-F1-S03 Test employee cart with only goods that has family sales price" {

    test WithFamilyPrices "it gets the regular price without the single view flag" {
        given:
            Web set_header "x-flow-id" "G1-F1-S03-step1-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S03/step1/request.json" as REQUEST
        when:
            # This would be the content of the request.json for G1-F1-S03 step1
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
            FileSystem read_file "examples/data/G1/F1/S03/step1/response.json" as RESPONSE
        then:
            Web response_status_is 200
            # Validate specific values
            Web json_path at "/salesChannel" equals "Web"
            # Validate item details
            Web json_path at "/customer/isEmployee" equals true
            Web response_body_equals_json "${RESPONSE}"
    }

    # The second step, which depends on the first.
    test WithSingleView "it gets the regular price with the single view flag" {
        given:
            Test has_succeeded WithFamilyPrices
            Web set_header "x-flow-id" "G1-F1-S03-step2-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S03/step2/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S03/step2/response.json" as RESPONSE
        when:
            # This would be the content of the request.json for G1-F1-S03 step2
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            # Validate specific values
            Web json_path at "/salesChannel" equals "Web"
            # Validate item details
            Web json_path at "/provideSingleView" equals true
            # Compare and validate with expected json
            Web response_body_equals_json "${RESPONSE}"
    }
    # The third step, which depends on the second.
    test WithoutSingleView "it gets the regular price with the single view flag set to false" {
        given:
            Test has_succeeded WithSingleView
            Web set_header "x-flow-id" "G1-F1-S03-step3-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S03/step3/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S03/step3/response.json" as RESPONSE
        when:
            # This would be the content of the request.json for G1-F1-S03 step3
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
            System log "Ignoring json fields"
        then:
            Web response_status_is 200
            # Validate specific values
            Web json_path at "/salesChannel" equals "Web"
            # Validate item details
            Web json_path at "/provideSingleView" equals false
            # Compare and validate with expected json
            Web response_body_equals_json "${RESPONSE}"
    }
}

parallel scenario "G1-F1-S35 ShouldFetchChildItems is Set to false and Cart is not valid" {
    test FaultyInput "it fails when input is wrong" {
        given:
            Web set_header "x-flow-id" "G1-F1-S35-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S35/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S35/response.json" as RESPONSE
        when:
            # This is the request for G1-F1-S35
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            # Client error
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }
}

parallel scenario "G1-F1-S73 Test cart validation scenarios" {

    test Step3ItemTypeMismatchNotHappen "Item type mismatch will not happen when shouldFetchChild is true" {
        given:
            Test can_start
            Web set_header "x-flow-id" "G1-F1-S73-step3-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step3/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step3/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            Web response_body_equals_json "${RESPONSE}"
    }

    test Step4ItemTypeMismatch "Item type mismatch validation" {
        given:
            Test has_succeeded Step3ItemTypeMismatchNotHappen
            Web set_header "x-flow-id" "G1-F1-S73-step4-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step4/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step4/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step5WrongChild "Wrong child validation" {
        given:
            Test has_succeeded Step4ItemTypeMismatch
            Web set_header "x-flow-id" "G1-F1-S73-step5-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step5/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step5/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step6ChildCountMismatch "Child item counts do not match validation" {
        given:
            Test has_succeeded Step5WrongChild
            Web set_header "x-flow-id" "G1-F1-S73-step6-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step6/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step6/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step7QuantityMismatch "Mismatch in quantity validation" {
        given:
            Test has_succeeded Step6ChildCountMismatch
            Web set_header "x-flow-id" "G1-F1-S73-step7-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step7/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step7/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step8InvalidBusinessUnit "Invalid business unit validation" {
        given:
            Test has_succeeded Step7QuantityMismatch
            Web set_header "x-flow-id" "G1-F1-S73-step8-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step8/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step8/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step9BusinessUnitPastValidTo "Business unit with past validTo" {
        given:
            Test has_succeeded Step8InvalidBusinessUnit
            Web set_header "x-flow-id" "G1-F1-S73-step9-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step9/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step9/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            Web response_body_equals_json "${RESPONSE}"
    }

    test Step10DeliveryServiceValidation "Delivery service number validation" {
        given:
            Test has_succeeded Step9BusinessUnitPastValidTo
            Web set_header "x-flow-id" "G1-F1-S73-step10-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step10/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step10/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step11ProvidedServiceValidation "Provided service number validation" {
        given:
            Test has_succeeded Step10DeliveryServiceValidation
            Web set_header "x-flow-id" "G1-F1-S73-step11-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step11/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step11/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step12ItemTypeMismatchNotHappenAgain "Item type mismatch will not happen when shouldFetchChild is true (second case)" {
        given:
            Test has_succeeded Step11ProvidedServiceValidation
            Web set_header "x-flow-id" "G1-F1-S73-step12-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step12/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step12/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            Web response_body_equals_json "${RESPONSE}"
    }

    test Step13InvalidCurrency "Invalid currency validation" {
        given:
            Test has_succeeded Step12ItemTypeMismatchNotHappenAgain
            Web set_header "x-flow-id" "G1-F1-S73-step13-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S73/step13/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S73/step13/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 400
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }
}

parallel scenario "G1-F1-S74 Test cart for SPR and child articles price mismatch" {

    test Step1SPRRegularSalesPrice "SPR with only regular sales price" {
        given:
            Test can_start
            Web set_header "x-flow-id" "G1-F1-S74-step1-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S74/step1/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S74/step1/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 500
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }

    test Step2SPRFamilySalesPrice "SPR with regular sales price and family sales price" {
        given:
            Test has_succeeded Step1SPRRegularSalesPrice
            Web set_header "x-flow-id" "G1-F1-S74-step2-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S74/step2/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S74/step2/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 500
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }
}

parallel scenario "G1-F1-S75 Test cart for articles with multiple valid prices of same price type" {

    test MultiplePricesValidation "Articles with multiple valid prices of same price type" {
        given:
            Test can_start
            Web set_header "x-flow-id" "G1-F1-S75-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S75/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S75/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 500
            Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
    }
}

parallel scenario "G1-F1-S76 Test cart with same SPR having Reference Items as true and false" {

    test SPRReferenceItems "Same SPR with different Reference Items settings" {
        given:
            Test can_start
            Web set_header "x-flow-id" "G1-F1-S76-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S76/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S76/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            Web response_body_equals_json "${RESPONSE}"
    }
}

parallel scenario "G1-F1-S77 Test cart with isFollowupMeeting variations" {

    test Step1FollowupMeetingViaFlag "isFollowupMeeting via flag" {
        given:
            Test can_start
            Web set_header "x-flow-id" "G1-F1-S77-step1-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S77/step1/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S77/step1/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            Web response_body_equals_json "${RESPONSE}"
    }

    test Step2FollowupMeetingViaEnum "isFollowupMeeting via enum" {
        given:
            Test has_succeeded Step1FollowupMeetingViaFlag
            Web set_header "x-flow-id" "G1-F1-S77-step2-${UUID}"
            FileSystem read_file "examples/data/G1/F1/S77/step2/request.json" as REQUEST
            FileSystem read_file "examples/data/G1/F1/S77/step2/response.json" as RESPONSE
        when:
            Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
        then:
            Web response_status_is 200
            Web response_body_equals_json "${RESPONSE}"
    }
}

# Running similar one-step scenarios in choreos foreach loop
scenario "Generated single-step 200 tests" {
    foreach SC in ${OK_SCENARIOS} {
        test "Generated_${SC}" "auto-generated test for ${SC}" {
            given:
                Web set_header "x-flow-id" "${SC}-${UUID}"
                FileSystem read_file "examples/data/G1/F1/${SC}/request.json" as REQUEST
                FileSystem read_file "examples/data/G1/F1/${SC}/response.json" as RESPONSE
            when:
                Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
            then:
                Web response_status_is 200
                Web response_body_equals_json "${RESPONSE}"
        }
    }
}

parallel scenario "Generated single-step 400 tests (ignore timestamp)" {
    foreach SC in ${BAD_TS_SCENARIOS} {
        test "Generated_${SC}" "auto-generated test for ${SC}" {
            given:
                Web set_header "x-flow-id" "${SC}-${UUID}"
                FileSystem read_file "examples/data/G1/F1/${SC}/request.json" as REQUEST
                FileSystem read_file "examples/data/G1/F1/${SC}/response.json" as RESPONSE
            when:
                Web http_post "${BASE_URL}/price-cart" with_body "${REQUEST}"
            then:
                Web response_status_is 400
                Web response_body_equals_json "${RESPONSE}" ignore_fields ["timestamp"]
        }
    }
}
```
