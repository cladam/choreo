// src/parser/parser.rs

use crate::error::AppError;
use crate::parser::ast::{
    Action, Condition, GivenStep, ReportFormat, Scenario, Statement, TestCase, TestSuite,
    TestSuiteSettings, Value,
};
use pest::iterators::{Pair, Pairs};
use pest::Parser;

#[derive(pest_derive::Parser)]
#[grammar = "parser/choreo.pest"]
pub struct ChoreoParser;

pub fn parse(source: &str) -> Result<TestSuite, AppError> {
    let pairs = ChoreoParser::parse(Rule::grammar, source)?;
    let statements = pairs
        .flat_map(|pair| pair.into_inner())
        .filter(|pair| pair.as_rule() != Rule::EOI)
        .map(parse_statement)
        .collect();
    Ok(TestSuite { statements })
}

fn parse_statement(pair: Pair<Rule>) -> Statement {
    match pair.as_rule() {
        Rule::settings_def => Statement::SettingsDef(parse_settings(pair.into_inner())),
        Rule::background_def => Statement::BackgroundDef(parse_background(pair.into_inner())),
        Rule::env_def => Statement::EnvDef(parse_env(pair.into_inner())),
        Rule::var_def => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let value = parse_value(inner.next().unwrap());
            Statement::VarDef(name, value)
        }
        Rule::actors_def => Statement::ActorDef(parse_actors(pair.into_inner())),
        Rule::feature_def => {
            let feature_name = parse_string(pair.into_inner().next().unwrap());
            Statement::FeatureDef(feature_name)
        }
        Rule::scenario_def => Statement::Scenario(parse_scenario(pair.into_inner())),
        _ => unreachable!("Unexpected top-level statement: {:?}", pair.as_rule()),
    }
}

fn parse_settings(pairs: Pairs<Rule>) -> TestSuiteSettings {
    let mut settings = TestSuiteSettings::default();
    for setting in pairs {
        let mut inner = setting.into_inner();
        let key = inner.next().unwrap().as_str();
        let value_pair = inner.next().unwrap();
        match key {
            "timeout_seconds" => {
                settings.timeout_seconds = value_pair
                    .as_str()
                    .parse()
                    .unwrap_or(settings.timeout_seconds)
            }
            "report_format" => {
                settings.report_format = match value_pair.as_str() {
                    "json" => ReportFormat::Json,
                    "junit" => ReportFormat::Junit,
                    "none" => ReportFormat::None,
                    _ => settings.report_format,
                }
            }
            "report_path" => settings.report_path = value_pair.as_str().to_string(),
            "stop_on_failure" => {
                settings.stop_on_failure = value_pair.as_str().parse().unwrap_or(false)
            }
            "shell_path" => settings.shell_path = Some(value_pair.as_str().to_string()),
            "expected_failures" => {
                settings.expected_failures = value_pair
                    .as_str()
                    .parse()
                    .unwrap_or(settings.expected_failures)
            }
            _ => {} // Ignore unknown settings
        }
    }
    settings
}

fn parse_background(pairs: Pairs<Rule>) -> Vec<GivenStep> {
    pairs
        .map(|p| match p.as_rule() {
            Rule::action => GivenStep::Action(parse_action(p)),
            Rule::condition => GivenStep::Condition(parse_condition(p)),
            _ => unreachable!(),
        })
        .collect()
}

fn parse_env(pairs: Pairs<Rule>) -> Vec<String> {
    pairs.map(|p| p.as_str().to_string()).collect()
}

fn parse_actors(pairs: Pairs<Rule>) -> Vec<String> {
    pairs.map(|p| p.as_str().to_string()).collect()
}

fn parse_scenario(pairs: Pairs<Rule>) -> Scenario {
    let mut name = String::new();
    let mut tests = Vec::new();
    let mut after = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::string => name = parse_string(pair),
            Rule::test => tests.push(parse_test(pair.into_inner())),
            Rule::after_block => after = parse_after_block(pair.into_inner()),
            _ => unreachable!(),
        }
    }

    Scenario { name, tests, after }
}

fn parse_test(pairs: Pairs<Rule>) -> TestCase {
    let mut name = String::new();
    let mut description = String::new();
    let mut given = Vec::new();
    let mut when = Vec::new();
    let mut then = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::identifier => name = pair.as_str().to_string(),
            Rule::string => description = parse_string(pair),
            Rule::given_block => given = parse_given_block(pair.into_inner()),
            Rule::when_block => when = parse_when_block(pair.into_inner()),
            Rule::then_block => then = parse_then_block(pair.into_inner()),
            _ => unreachable!(),
        }
    }

    TestCase {
        name,
        description,
        given,
        when,
        then,
    }
}

fn parse_given_block(pairs: Pairs<Rule>) -> Vec<GivenStep> {
    pairs
        .map(|p| match p.as_rule() {
            Rule::action => GivenStep::Action(parse_action(p)),
            Rule::condition => GivenStep::Condition(parse_condition(p)),
            _ => unreachable!(),
        })
        .collect()
}

fn parse_when_block(pairs: Pairs<Rule>) -> Vec<Action> {
    pairs.map(parse_action).collect()
}

fn parse_then_block(pairs: Pairs<Rule>) -> Vec<Condition> {
    pairs.map(parse_condition).collect()
}

fn parse_after_block(pairs: Pairs<Rule>) -> Vec<Action> {
    pairs.map(parse_action).collect()
}

fn parse_condition(pair: Pair<Rule>) -> Condition {
    let inner_pair = pair.into_inner().next().unwrap();
    match inner_pair.as_rule() {
        Rule::wait_condition => {
            let mut inner = inner_pair.into_inner();
            let op = inner.next().unwrap().as_str().to_string();
            let wait_str = inner.next().unwrap().as_str();
            let wait = wait_str[..wait_str.len() - 1].parse::<f32>().unwrap_or(0.0);
            Condition::Wait { op, wait }
        }
        Rule::state_condition => {
            let outcome = inner_pair.into_inner().next().unwrap().as_str().to_string();
            Condition::StateSucceeded { outcome }
        }
        Rule::terminal_condition => {
            parse_terminal_condition(inner_pair.into_inner().next().unwrap())
        }
        Rule::filesystem_condition => {
            parse_filesystem_condition(inner_pair.into_inner().next().unwrap())
        }
        Rule::web_condition => parse_web_condition(inner_pair.into_inner().next().unwrap()),
        _ => unreachable!("Unhandled condition: {:?}", inner_pair.as_rule()),
    }
}

fn parse_terminal_condition(pair: Pair<Rule>) -> Condition {
    let actor = "Terminal".to_string(); // Hardcoded for now
    match pair.as_rule() {
        Rule::output_contains_condition => {
            let text = parse_string(pair.into_inner().next().unwrap());
            Condition::OutputContains { actor, text }
        }
        Rule::output_matches_condition => {
            let mut inner = pair.into_inner();
            let regex = parse_string(inner.next().unwrap());
            let capture_as = inner.next().map(|p| p.as_str().to_string());
            Condition::OutputMatches {
                actor,
                regex,
                capture_as,
            }
        }
        Rule::last_command_succeeded_cond => Condition::LastCommandSucceeded,
        Rule::last_command_failed_cond => Condition::LastCommandFailed,
        Rule::last_command_exit_code_is_cond => {
            let code = pair
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .parse::<i32>()
                .unwrap();
            Condition::LastCommandExitCodeIs(code)
        }
        Rule::stdout_is_empty_condition => Condition::StdoutIsEmpty,
        Rule::stderr_is_empty_condition => Condition::StderrIsEmpty,
        Rule::stderr_contains_condition => {
            let text = parse_string(pair.into_inner().next().unwrap());
            Condition::StderrContains(text)
        }
        Rule::output_starts_with_condition => {
            let text = parse_string(pair.into_inner().next().unwrap());
            Condition::OutputStartsWith(text)
        }
        Rule::output_ends_with_condition => {
            let text = parse_string(pair.into_inner().next().unwrap());
            Condition::OutputEndsWith(text)
        }
        Rule::output_equals_condition => {
            let text = parse_string(pair.into_inner().next().unwrap());
            Condition::OutputEquals(text)
        }
        Rule::output_is_valid_json_condition => Condition::OutputIsValidJson,
        Rule::json_output_has_path_condition => {
            let path = parse_string(pair.into_inner().next().unwrap());
            Condition::JsonOutputHasPath { path }
        }
        Rule::json_output_at_equals_condition => {
            let mut inner = pair.into_inner();
            let path = parse_string(inner.next().unwrap());
            let value = parse_value(inner.next().unwrap());
            Condition::JsonOutputAtEquals { path, value }
        }
        Rule::json_output_at_includes_condition => {
            let mut inner = pair.into_inner();
            let path = parse_string(inner.next().unwrap());
            let value = parse_value(inner.next().unwrap());
            Condition::JsonOutputAtIncludes { path, value }
        }
        Rule::json_output_at_has_item_count_condition => {
            let mut inner = pair.into_inner();
            let path = parse_string(inner.next().unwrap());
            let count = inner.next().unwrap().as_str().parse::<i32>().unwrap();
            Condition::JsonOutputAtHasItemCount { path, count }
        }
        _ => unreachable!("Unhandled terminal condition: {:?}", pair.as_rule()),
    }
}

fn parse_filesystem_condition(pair: Pair<Rule>) -> Condition {
    match pair.as_rule() {
        Rule::filesystem_condition_keyword => {
            let keyword = pair.as_str();
            let mut inner = pair.clone().into_inner();
            let path = parse_string(inner.next().unwrap());
            match keyword {
                "file_exists" => Condition::FileExists { path },
                "file_does_not_exist" => Condition::FileDoesNotExist { path },
                "dir_exists" => Condition::DirExists { path },
                "dir_does_not_exist" => Condition::DirDoesNotExist { path },
                "file_contains" => {
                    let content = parse_string(inner.next().unwrap());
                    Condition::FileContains { path, content }
                }
                _ => unreachable!("Unhandled filesystem keyword: {}", keyword),
            }
        }
        Rule::file_is_empty_condition => {
            let path = parse_string(pair.into_inner().next().unwrap());
            Condition::FileIsEmpty { path }
        }
        Rule::file_is_not_empty_condition => {
            let path = parse_string(pair.into_inner().next().unwrap());
            Condition::FileIsNotEmpty { path }
        }
        _ => {
            // This handles the case where the rule is nested, e.g., `filesystem_condition -> (filesystem_condition_keyword ...)`
            let mut inner = pair.into_inner();
            let keyword_pair = inner.next().unwrap();
            let keyword = keyword_pair.as_str();
            let mut keyword_inner = keyword_pair.into_inner();
            let path = parse_string(keyword_inner.next().unwrap());

            match keyword {
                "file_exists" => Condition::FileExists { path },
                "file_does_not_exist" => Condition::FileDoesNotExist { path },
                "dir_exists" => Condition::DirExists { path },
                "dir_does_not_exist" => Condition::DirDoesNotExist { path },
                "file_contains" => {
                    let content = parse_string(keyword_inner.next().unwrap());
                    Condition::FileContains { path, content }
                }
                _ => unreachable!("Unhandled filesystem keyword: {}", keyword),
            }
        }
    }
}

fn parse_web_condition(pair: Pair<Rule>) -> Condition {
    match pair.as_rule() {
        Rule::response_status_is_condition => {
            let status = pair
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .parse::<u16>()
                .unwrap();
            Condition::ResponseStatusIs(status)
        }
        Rule::response_body_contains_condition => {
            let value = parse_string(pair.into_inner().next().unwrap());
            Condition::ResponseBodyContains { value }
        }
        Rule::response_body_matches_condition => {
            let mut inner = pair.into_inner();
            let regex = parse_string(inner.next().unwrap());
            let capture_as = inner.next().map(|p| p.as_str().to_string());
            Condition::ResponseBodyMatches { regex, capture_as }
        }
        Rule::json_body_has_path_condition => {
            let path = parse_string(pair.into_inner().next().unwrap());
            Condition::JsonBodyHasPath { path }
        }
        Rule::json_path_equals_condition => {
            let mut inner = pair.into_inner();
            let path = parse_string(inner.next().unwrap());
            let expected_value = parse_value(inner.next().unwrap());
            Condition::JsonPathEquals {
                path,
                expected_value,
            }
        }
        _ => unreachable!("Unhandled web condition: {:?}", pair.as_rule()),
    }
}

fn parse_action(pair: Pair<Rule>) -> Action {
    let action_type = pair.into_inner().next().unwrap();
    match action_type.as_rule() {
        Rule::type_action => {
            let mut inner = action_type.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let content = parse_string(inner.next().unwrap());
            Action::Type { actor, content }
        }
        Rule::press_action => {
            let mut inner = action_type.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let key = parse_string(inner.next().unwrap());
            Action::Press { actor, key }
        }
        Rule::run_action => {
            let mut inner = action_type.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let command = parse_string(inner.next().unwrap());
            Action::Run { actor, command }
        }
        Rule::filesystem_action => {
            let mut inner = action_type.into_inner();
            let keyword = inner.next().unwrap().as_str();
            let path = parse_string(inner.next().unwrap());
            let content = inner.next().map(parse_string);

            match keyword {
                "create_file" => Action::CreateFile {
                    path,
                    content: content.unwrap_or_default(),
                },
                "create_dir" => Action::CreateDir { path },
                "delete_file" => Action::DeleteFile { path },
                "delete_dir" => Action::DeleteDir { path },
                _ => unreachable!("Unhandled filesystem action keyword: {}", keyword),
            }
        }
        Rule::web_action => {
            let mut inner = action_type.into_inner();
            let actor = inner.next().unwrap().as_str().to_string();
            let url = parse_string(inner.next().unwrap());
            Action::HttpGet { actor, url }
        }
        _ => unreachable!("Unhandled action: {:?}", action_type.as_rule()),
    }
}

fn parse_string(pair: Pair<Rule>) -> String {
    let inner = pair.into_inner().next().unwrap();
    inner.as_str().to_string()
}

fn parse_value(pair: Pair<Rule>) -> Value {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::string => Value::String(parse_string(inner)),
        Rule::number => Value::Number(inner.as_str().parse().unwrap()),
        _ => unreachable!(),
    }
}
