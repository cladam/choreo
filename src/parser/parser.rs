// parser.rs
use crate::parser::ast::{
    Action, Condition, GivenStep, ReportFormat, Scenario, ScenarioSpan, SettingSpan, Span,
    Statement, TestCase, TestCaseSpan, TestSuite, TestSuiteSettings, Value,
};
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser, Debug)]
#[grammar = "parser/choreo.pest"]
pub struct ChoreoParser;

/// Parses a source string into an Abstract Syntax Tree (AST).
pub fn parse(source: &str) -> Result<TestSuite, pest::error::Error<Rule>> {
    let pairs = ChoreoParser::parse(Rule::grammar, source)?.next().unwrap();

    let mut statements = vec![];
    for statement_pair in pairs.into_inner() {
        if statement_pair.as_rule() == Rule::EOI {
            break;
        }
        //println!("Parsed statement: {:?}", statement_pair);

        // This opens the "statement" container to get the content.
        let pair = statement_pair.into_inner().next().unwrap();
        //println!("Parsed pair: {:?}", pair);

        statements.push(match pair.as_rule() {
            // Now this will correctly match on the inner rule.
            Rule::actors_def => build_actors_def(pair),
            Rule::settings_def => build_settings_def(pair),
            Rule::env_def => build_env_def(pair),
            Rule::var_def => build_var_def(pair),
            Rule::feature_def => build_feature_def(pair),
            Rule::scenario_def => build_scenario(pair),
            Rule::background_def => build_background_def(pair),
            _ => unimplemented!("Parser rule not handled: {:?}", pair.as_rule()),
        });
    }

    Ok(TestSuite { statements })
}

// Helper function for a background definition.
fn build_background_def(pair: Pair<Rule>) -> Statement {
    let steps = build_given_steps(pair.into_inner());
    Statement::BackgroundDef(steps)
}

// Helper function for a simple definition.
fn build_actors_def(pair: Pair<Rule>) -> Statement {
    let identifiers: Vec<String> = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::identifier)
        .map(|p| p.as_str().to_string())
        .collect();
    Statement::ActorDef(identifiers)
}

// Helper function for settings
fn build_settings_def(pair: Pair<Rule>) -> Statement {
    let span = pair.as_span();
    let mut settings = TestSuiteSettings::default();
    let mut setting_spans = SettingSpan {
        timeout_seconds_span: None,
        report_path_span: None,
        report_format_span: None,
        shell_path_span: None,
        stop_on_failure_span: None,
        expected_failures_span: None,
    };

    // Store the span information
    settings.span = Some(Span {
        start: span.start(),
        end: span.end(),
        line: span.start_pos().line_col().0,
        column: span.start_pos().line_col().1,
    });

    for setting_pair in pair.into_inner() {
        let setting_span = setting_pair.as_span();
        let mut inner = setting_pair.into_inner();
        let key = inner.next().unwrap().as_str();
        let value_pair = inner.next().unwrap();

        let span_info = Span {
            start: setting_span.start(),
            end: setting_span.end(),
            line: setting_span.start_pos().line_col().0,
            column: setting_span.start_pos().line_col().1,
        };

        match key {
            "timeout_seconds" => {
                setting_spans.timeout_seconds_span = Some(span_info);
                if let Value::Number(n) = build_value(value_pair) {
                    settings.timeout_seconds = n as u64;
                } else {
                    panic!("'timeout_seconds' setting must be a number");
                }
            }
            "report_path" => {
                setting_spans.report_path_span = Some(span_info);
                if let Value::String(s) = build_value(value_pair) {
                    if s.trim().is_empty() {
                        panic!(
                            "'report_path' setting cannot be an empty or whitespace-only string."
                        );
                    }
                    settings.report_path = s;
                } else {
                    panic!("'report_path' setting must be a string");
                }
            }
            "report_format" => {
                setting_spans.report_format_span = Some(span_info);
                if let Value::String(s) = build_value(value_pair) {
                    settings.report_format = match s.as_str() {
                        "json" => ReportFormat::Json,
                        "junit" => ReportFormat::Junit,
                        _ => panic!("Invalid 'report_format': must be 'json' or 'junit'"),
                    };
                } else {
                    panic!("'report_format' setting must be a string");
                }
            }
            "stop_on_failure" => {
                setting_spans.stop_on_failure_span = Some(span_info);
                if value_pair.as_rule() == Rule::binary_op {
                    settings.stop_on_failure = value_pair.as_str().parse().unwrap();
                } else {
                    panic!("'stop_on_failure' setting must be a boolean (true/false)");
                }
            }
            "shell_path" => {
                setting_spans.shell_path_span = Some(span_info);
                if let Value::String(s) = build_value(value_pair) {
                    if s.trim().is_empty() {
                        panic!(
                            "'shell_path' setting cannot be an empty or whitespace-only string."
                        );
                    }
                    settings.shell_path = Some(s);
                } else {
                    panic!("'shell_path' setting must be a string");
                }
            }
            "expected_failures" => {
                setting_spans.expected_failures_span = Some(span_info);
                if let Value::Number(n) = build_value(value_pair) {
                    settings.expected_failures = n as usize;
                } else {
                    panic!("'expected_failures' setting must be a number");
                }
            }
            _ => { /* Ignore unknown settings */ }
        }
    }

    // Only set setting_spans if at least one field is Some
    settings.setting_spans = Some(setting_spans);
    Statement::SettingsDef(settings)
}

// Helper function for a var definition.
fn build_var_def(pair: Pair<Rule>) -> Statement {
    let mut inner = pair.into_inner();
    let key = inner.next().unwrap().as_str().to_string();
    let value_pair = inner.next().unwrap();
    let value = build_value(value_pair);
    Statement::VarDef(key, value)
}

// Helper function for a feature definition.
fn build_feature_def(pair: Pair<Rule>) -> Statement {
    let name = pair
        .into_inner()
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string();
    Statement::FeatureDef(name)
}

fn build_env_def(pair: Pair<Rule>) -> Statement {
    let identifiers: Vec<String> = pair.into_inner().map(|p| p.as_str().to_string()).collect();
    Statement::EnvDef(identifiers)
}

// This is the other key function to implement.
fn build_actions(pairs: Pairs<Rule>) -> Vec<Action> {
    pairs
        .map(|pair| {
            // An 'action' rule from the grammar contains one of the specific action types.
            let inner_action = pair.into_inner().next().unwrap();
            build_action(inner_action)
        })
        .collect()
}

fn build_scenario(pair: Pair<Rule>) -> Statement {
    let span = pair.as_span();
    let mut inner = pair.into_inner();
    let mut scenario = Scenario::default();
    let mut scenario_spans = ScenarioSpan {
        name_span: None,
        tests_span: None,
        after_span: None,
    };

    scenario.span = Some(Span {
        start: span.start(),
        end: span.end(),
        line: span.start_pos().line_col().0,
        column: span.start_pos().line_col().1,
    });

    // Peek and look for the parallel keyword
    if let Some(token) = inner.peek() {
        if token.as_rule() == Rule::parallel_keyword {
            scenario.parallel = true;
            inner.next();
        }
    }

    // The first inner pair is the scenario name (a string).
    let name_pair = inner.next().unwrap();
    scenario_spans.name_span = Some(Span {
        start: name_pair.as_span().start(),
        end: name_pair.as_span().end(),
        line: name_pair.as_span().start_pos().line_col().0,
        column: name_pair.as_span().start_pos().line_col().1,
    });
    scenario.name = unescape_string(name_pair.into_inner().next().unwrap().as_str());

    for item in inner {
        let item_span = item.as_span();
        let span_info = Span {
            start: item_span.start(),
            end: item_span.end(),
            line: item_span.start_pos().line_col().0,
            column: item_span.start_pos().line_col().1,
        };

        match item.as_rule() {
            Rule::test => {
                scenario_spans.tests_span = Some(span_info);
                scenario.tests.push(build_test_case(item));
            }
            Rule::after_block => {
                scenario_spans.after_span = Some(span_info);
                scenario.after = build_actions(item.into_inner());
            }
            _ => {}
        }
    }

    scenario.scenario_span = Some(scenario_spans);
    Statement::Scenario(scenario)
}

/// Builds a TestCase from a parsed Pair.
pub fn build_test_case(pair: Pair<Rule>) -> TestCase {
    let span = pair.as_span();
    let mut inner = pair.into_inner();
    let mut testcase_spans = TestCaseSpan {
        name_span: None,
        description_span: None,
        given_span: None,
        when_span: None,
        then_span: None,
    };

    let name_pair = inner.next().unwrap();
    let name = name_pair.as_str().to_string();
    testcase_spans.name_span = Some(Span {
        start: name_pair.as_span().start(),
        end: name_pair.as_span().end(),
        line: name_pair.as_span().start_pos().line_col().0,
        column: name_pair.as_span().start_pos().line_col().1,
    });

    let description_pair = inner.next().unwrap();
    let description_span = description_pair.as_span();
    let description = description_pair
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string();
    testcase_spans.description_span = Some(Span {
        start: description_span.start(),
        end: description_span.end(),
        line: description_span.start_pos().line_col().0,
        column: description_span.start_pos().line_col().1,
    });

    let given_block = inner.next().expect("Missing given block in test case");
    let given_span = given_block.as_span();
    testcase_spans.given_span = Some(Span {
        start: given_span.start(),
        end: given_span.end(),
        line: given_span.start_pos().line_col().0,
        column: given_span.start_pos().line_col().1,
    });

    let when_block = inner.next().expect("Missing when block in test case");
    let when_span = when_block.as_span();
    testcase_spans.when_span = Some(Span {
        start: when_span.start(),
        end: when_span.end(),
        line: when_span.start_pos().line_col().0,
        column: when_span.start_pos().line_col().1,
    });

    let then_block = inner.next().expect("Missing then block in test case");
    let then_span = then_block.as_span();
    testcase_spans.then_span = Some(Span {
        start: then_span.start(),
        end: then_span.end(),
        line: then_span.start_pos().line_col().0,
        column: then_span.start_pos().line_col().1,
    });

    TestCase {
        name,
        description,
        given: build_given_steps(given_block.into_inner()),
        when: build_actions(when_block.into_inner()),
        then: build_conditions(then_block.into_inner()),
        span: Some(Span {
            start: span.start(),
            end: span.end(),
            line: span.start_pos().line_col().0,
            column: span.start_pos().line_col().1,
        }),
        testcase_spans: Some(testcase_spans),
    }
}

// Builds a vector of GivenSteps, which can be either an Action or a Condition.
pub fn build_given_steps(pairs: Pairs<Rule>) -> Vec<GivenStep> {
    pairs
        .map(|pair| {
            // The pair will be either an 'action' or a 'condition'.
            match pair.as_rule() {
                Rule::action => {
                    // The 'action' rule contains one of the specific action types.
                    let specific_action = pair.into_inner().next().unwrap();
                    GivenStep::Action(build_action(specific_action))
                }
                Rule::condition => GivenStep::Condition(build_condition(pair)),
                _ => unreachable!("Unexpected rule in given block: {:?}", pair.as_rule()),
            }
        })
        .collect()
}

/// Builds a single Condition from a specific, inner condition rule Pair.
/// This is a helper to avoid unwrapping the 'condition' rule multiple times.
pub fn build_condition_from_specific(inner_cond: Pair<Rule>) -> Condition {
    //println!("Building condition from specific: {:?}", inner_cond);
    match inner_cond.as_rule() {
        Rule::wait_condition => {
            let mut inner = inner_cond.into_inner();
            let op = inner.next().unwrap().as_str().to_string();
            let wait_marker_str = inner.next().unwrap().as_str();

            let wait = if wait_marker_str.ends_with("ms") {
                let value_str = &wait_marker_str[..wait_marker_str.len() - 2];
                value_str.parse::<f32>().unwrap() / 1000.0
            } else if wait_marker_str.ends_with('s') {
                let value_str = &wait_marker_str[..wait_marker_str.len() - 1];
                value_str.parse::<f32>().unwrap()
            } else {
                // This case should not be reached if the grammar is correct
                0.0
            };
            Condition::Wait { op, wait }
        }
        Rule::terminal_condition => {
            let mut inner = inner_cond.into_inner();
            let terminal_cond = inner.next().unwrap();
            build_condition_from_specific(terminal_cond)
        }
        Rule::output_contains_condition => {
            let mut inner = inner_cond.into_inner();
            //let actor = inner.next().unwrap().as_str().to_string();
            let actor = "Terminal".to_string(); // Default actor for terminal conditions
            let text = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::OutputContains { actor, text }
        }
        Rule::state_condition => {
            let outcome = inner_cond.into_inner().next().unwrap().as_str().to_string();
            Condition::StateSucceeded { outcome }
        }
        Rule::output_matches_condition => {
            let mut inner = inner_cond.into_inner();
            //let actor = inner.next().unwrap().as_str().to_string();
            let regex = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            let capture_as = inner.next().map(|p| p.as_str().to_string());
            Condition::OutputMatches {
                actor: "Terminal".to_string(),
                regex,
                capture_as,
            }
        }
        Rule::last_command_succeeded_cond => Condition::LastCommandSucceeded,
        Rule::last_command_failed_cond => Condition::LastCommandFailed,
        Rule::last_command_exit_code_is_cond => {
            let mut inner = inner_cond.into_inner();
            let code_str = inner.next().unwrap().as_str();
            let code: i32 = code_str.parse().unwrap();
            Condition::LastCommandExitCodeIs(code)
        }
        Rule::output_is_valid_json_condition => Condition::OutputIsValidJson,
        Rule::json_output_has_path_condition => {
            let mut inner = inner_cond.into_inner();
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::JsonOutputHasPath { path }
        }
        Rule::json_output_at_equals_condition => {
            let mut inner = inner_cond.into_inner();
            let path = inner.next().unwrap().as_str().to_string();
            let value = build_value(inner.next().unwrap());
            Condition::JsonOutputAtEquals { path, value }
        }
        Rule::json_output_at_includes_condition => {
            let mut inner = inner_cond.into_inner();
            let path = inner.next().unwrap().as_str().to_string();
            let value = build_value(inner.next().unwrap());
            Condition::JsonOutputAtIncludes { path, value }
        }
        Rule::json_output_at_has_item_count_condition => {
            let mut inner = inner_cond.into_inner();
            let path = inner.next().unwrap().as_str().to_string();
            let count_str = inner.next().unwrap().as_str();
            let count: i32 = count_str.parse().unwrap();
            Condition::JsonOutputAtHasItemCount { path, count }
        }
        Rule::file_is_empty_condition => {
            let mut inner = inner_cond.into_inner();
            let path = unescape_string(inner.next().unwrap().into_inner().next().unwrap().as_str());
            Condition::FileIsEmpty { path }
        }
        Rule::file_is_not_empty_condition => {
            let mut inner = inner_cond.into_inner();
            let path = unescape_string(inner.next().unwrap().into_inner().next().unwrap().as_str());
            Condition::FileIsNotEmpty { path }
        }
        Rule::filesystem_condition => {
            let mut inner = inner_cond.into_inner();
            let next_pair = inner.next().unwrap();

            // Handle the different structures within a filesystem_condition
            match next_pair.as_rule() {
                Rule::file_is_empty_condition => build_condition_from_specific(next_pair),
                Rule::file_is_not_empty_condition => build_condition_from_specific(next_pair),
                _ => {
                    // This handles `filesystem_condition_keyword ~ string ...`
                    let keyword = next_pair.as_str();
                    let path = inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();

                    match keyword {
                        "file_exists" => Condition::FileExists { path },
                        "file_does_not_exist" => Condition::FileDoesNotExist { path },
                        "dir_exists" => Condition::DirExists { path },
                        "dir_does_not_exist" => Condition::DirDoesNotExist { path },
                        "file_contains" => {
                            let content = inner
                                .next()
                                .unwrap()
                                .into_inner()
                                .next()
                                .unwrap()
                                .as_str()
                                .to_string();
                            Condition::FileContains { path, content }
                        }
                        _ => unreachable!("Unsupported filesystem condition keyword: {}", keyword),
                    }
                }
            }
        }
        Rule::stdout_is_empty_condition => Condition::StdoutIsEmpty,
        Rule::stderr_is_empty_condition => Condition::StderrIsEmpty,
        Rule::stderr_contains_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            Condition::StderrContains(text)
        }
        Rule::output_starts_with_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            //println!("Building output_starts_with_condition '{}'", text);
            Condition::OutputStartsWith(text)
        }
        Rule::output_ends_with_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            Condition::OutputEndsWith(text)
        }
        Rule::output_equals_condition => {
            let text = unescape_string(
                inner_cond
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            );
            Condition::OutputEquals(text)
        }
        Rule::web_condition => {
            let inner = inner_cond.into_inner().next().unwrap();
            build_condition_from_specific(inner)
        }
        Rule::response_status_is_condition => {
            let status = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .parse()
                .unwrap();
            Condition::ResponseStatusIs(status)
        }
        Rule::response_status_is_success_condition => Condition::ResponseStatusIsSuccess,
        Rule::response_status_is_error_condition => Condition::ResponseStatusIsError,
        Rule::response_status_is_in_condition => {
            let statuses: Vec<u16> = inner_cond
                .into_inner()
                .filter(|p| p.as_rule() == Rule::number)
                .map(|p| p.as_str().parse().unwrap())
                .collect();
            Condition::ResponseStatusIsIn(statuses)
        }
        Rule::response_time_is_below_condition => {
            let mut inner = inner_cond.into_inner();
            let duration_marker_str = inner.next().unwrap().as_str();

            let duration = if duration_marker_str.ends_with("ms") {
                let value_str = &duration_marker_str[..duration_marker_str.len() - 2];
                value_str.parse::<f32>().unwrap() / 1000.0
            } else if duration_marker_str.ends_with('s') {
                let value_str = &duration_marker_str[..duration_marker_str.len() - 1];
                value_str.parse::<f32>().unwrap()
            } else {
                // This case should not be reached if the grammar is correct
                0.0
            };

            Condition::ResponseTimeIsBelow { duration }
        }
        Rule::response_body_contains_condition => {
            let value = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::ResponseBodyContains { value }
        }
        Rule::response_body_matches_condition => {
            let mut inner = inner_cond.into_inner();
            let regex_str = inner.next().unwrap().into_inner().next().unwrap().as_str();
            let regex = unescape_string(regex_str);
            let capture_as = inner.next().map(|p| p.as_str().to_string());
            Condition::ResponseBodyMatches { regex, capture_as }
        }
        Rule::response_body_equals_json => {
            let expected = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .trim_matches('"')
                .to_string();
            Condition::ResponseBodyEqualsJson { expected }
        }
        Rule::json_value_is_string_condition => {
            let path = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::JsonValueIsString { path }
        }
        Rule::json_value_is_number_condition => {
            let path = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::JsonValueIsNumber { path }
        }
        Rule::json_value_is_array_condition => {
            let path = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::JsonValueIsArray { path }
        }
        Rule::json_value_is_object_condition => {
            let path = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::JsonValueIsObject { path }
        }
        Rule::json_value_has_size_condition => {
            let mut inner = inner_cond.into_inner();
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            let size_str = inner.next().unwrap().as_str();
            let size: usize = size_str.parse().unwrap();
            Condition::JsonValueHasSize { path, size }
        }
        Rule::json_body_has_path_condition => {
            let path = inner_cond
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            Condition::JsonBodyHasPath { path }
        }
        Rule::json_path_equals_condition => {
            let mut inner = inner_cond.into_inner();
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            let expected_value = build_value(inner.next().unwrap());
            Condition::JsonPathEquals {
                path,
                expected_value,
            }
        }
        _ => unreachable!("Unhandled condition: {:?}", inner_cond.as_rule()),
    }
}

/// Builds a single Condition from a parsed Pair.
pub fn build_condition(pair: Pair<Rule>) -> Condition {
    // A 'condition' pair from the grammar contains one of the specific condition types.
    let inner_cond = pair.into_inner().next().unwrap();
    build_condition_from_specific(inner_cond)
}

/// Builds a vector of Conditions from parsed Pairs.
fn build_conditions(pairs: Pairs<Rule>) -> Vec<Condition> {
    pairs.map(build_condition).collect()
}

// --- Single Item Build Functions ---

/// Builds a single Action from a parsed Pair.
pub fn build_action(inner_action: Pair<Rule>) -> Action {
    //println!("Building action for inner_action: {:?}", inner_action);
    match inner_action.as_rule() {
        Rule::type_action => {
            let mut inner = inner_action.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let content = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());
            Action::Type { actor, content }
        }
        Rule::press_action => {
            let mut inner = inner_action.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let key = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());
            Action::Press { actor, key }
        }
        Rule::run_action => {
            let mut inner = inner_action.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let command = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());
            println!("Terminal runs: {}", command);
            Action::Run { actor, command }
        }
        Rule::filesystem_action => {
            let mut inner = inner_action.into_inner();
            //let _actor = inner.next().unwrap().as_str(); // Consume the actor identifier
            let keyword = inner.next().unwrap().as_str();
            let path = inner
                .next()
                .unwrap()
                .into_inner()
                .next()
                .map_or(String::new(), |p| p.as_str().to_string());

            match keyword {
                "create_dir" => Action::CreateDir { path },
                "delete_file" => Action::DeleteFile { path },
                "delete_dir" => Action::DeleteDir { path },
                "create_file" => {
                    let content = if let Some(content_pair) = inner.next() {
                        content_pair
                            .into_inner()
                            .next()
                            .map_or(String::new(), |p| p.as_str().to_string())
                    } else {
                        String::new()
                    };
                    Action::CreateFile { path, content }
                }
                "read_file" => {
                    let variable = inner.next().map(|p| p.as_str().to_string());
                    //println!("FS ReadFile Path: {}", path);
                    Action::ReadFile { path, variable }
                }
                _ => unreachable!(),
            }
        }
        Rule::web_action => {
            let mut inner = inner_action.into_inner();
            let _actor = inner.next().unwrap().as_str().to_string();
            // The next pair determines the specific web action type.
            let action_type = inner.next().unwrap();
            let action_type_str = action_type.as_str();
            // The action_type will have its own inner structure.

            let mut action_inner = action_type.into_inner();
            let method = action_type_str.split_whitespace().next().unwrap_or("");
            //println!("Building web action method: {}", method);

            match method {
                "set_header" => {
                    let key = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    let value = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    Action::HttpSetHeader { key, value }
                }
                "clear_header" => {
                    let key = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    Action::HttpClearHeader { key }
                }
                "clear_headers" => Action::HttpClearHeaders,
                "set_cookie" => {
                    let key = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    let value = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    Action::HttpSetCookie { key, value }
                }
                "clear_cookie" => {
                    let key = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    Action::HttpClearCookie { key }
                }
                "clear_cookies" => Action::HttpClearCookies,
                "http_get" => {
                    let url = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    Action::HttpGet { url }
                }
                "http_post" => {
                    let url = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    let body = unescape_string(
                        action_inner
                            .next()
                            .unwrap()
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str(),
                    );
                    Action::HttpPost { url, body }
                }
                "http_put" => {
                    let url = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    let body = unescape_string(
                        action_inner
                            .next()
                            .unwrap()
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str(),
                    );
                    Action::HttpPut { url, body }
                }
                "http_patch" => {
                    let url = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    let body = unescape_string(
                        action_inner
                            .next()
                            .unwrap()
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str(),
                    );
                    Action::HttpPatch { url, body }
                }
                "http_delete" => {
                    let url = action_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    Action::HttpDelete { url }
                }
                // ... other methods
                _ => panic!("Unknown action method: {}", method),
            }
        }
        _ => unreachable!("Unhandled action: {:?}", inner_action.as_rule()),
    }
}

// The grammar is `identifier ~ "http_get" ~ string`, so "http_get" is implicit.
//             let url = inner
//                 .next()
//                 .unwrap()
//                 .into_inner()
//                 .next()
//                 .unwrap()
//                 .as_str()
//                 .to_string();
//             Action::HttpGet { url }
//         }
//         _ => unreachable!("Unhandled action: {:?}", inner_action.as_rule()),
//     }
// }

fn build_value(pair: Pair<Rule>) -> Value {
    // The `value` rule is silent, so we need to inspect its inner pair.
    let inner_pair = pair.clone().into_inner().next().unwrap();
    //println!("{:?}", inner_pair);
    match inner_pair.as_rule() {
        Rule::string => {
            let inner = inner_pair.into_inner().next().unwrap();
            Value::String(unescape_string(inner.as_str()))
        }
        Rule::number => Value::Number(inner_pair.as_str().parse().unwrap()),
        Rule::identifier => {
            // Handle variable references - convert identifier to a placeholder string
            let var_name = pair.as_str();
            Value::String(format!("${{{}}}", var_name))
        }
        _ => {
            println!("{:?}", pair);
            unreachable!("Unexpected value rule: {:?}", pair.as_rule())
        }
    }
}

/// Unescapes a string captured by the parser.
fn unescape_string(s: &str) -> String {
    s.replace("\\\"", "\"")
        .replace("\\'", "'")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace("\\\\", "\\")
}
