use crate::gitql_schema::{tables_fields_names, tables_fields_types};
// use crate::nushell_render::render_objects;
// use nu_path::expand_path_with;
use nu_plugin::{serve_plugin, MsgPackSerializer, Plugin, PluginCommand};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, SyntaxShape, Value};
// use atty::Stream;
use gitql_cli::{
    // arguments,
    arguments::{Arguments, OutputFormat},
    diagnostic_reporter,
    diagnostic_reporter::DiagnosticReporter,
    // render,
};
use gitql_core::{environment::Environment, schema::Schema};
use gitql_data_provider::GitDataProvider;
use gitql_engine::{data_provider::DataProvider, engine, engine::EvaluationResult::SelectedGroups};
use gitql_parser::diagnostic::Diagnostic;
use gitql_parser::{parser, tokenizer};
use gitql_std::aggregation::{aggregation_function_signatures, aggregation_functions};

mod gitql_data_provider;
mod gitql_functions;
mod gitql_schema;
mod nushell_render;

pub struct GitqlPlugin;

impl Plugin for GitqlPlugin {
    fn version(&self) -> String {
        // This automatically uses the version of your package from Cargo.toml as the plugin version
        // sent to Nushell
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            // Commands should be added here
            Box::new(Gitql),
        ]
    }
}

pub struct Gitql;

impl SimplePluginCommand for Gitql {
    type Plugin = GitqlPlugin;

    fn name(&self) -> &str {
        "gitql"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            // .input_output_types(vec![(Type::Nothing, Type::String)])
            // --json flag
            // --csv flag
            // --debug/analysis flag
            // --repo folder
            .required("query", SyntaxShape::String, "gitql query string")
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Use gitql to query git repositories"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "gitql 'show tables'",
                description: "Show the tables available to be queried",
                result: None,
            },
            Example {
                example: "gitql 'select * from refs limit 10'",
                description: "Show the first 10 refs",
                result: None,
            },
            Example {
                example: "gitql 'desribe commits'",
                description: "Show the data types of the fields in the commits table",
                result: None,
            },
            Example {
                example: r#"gitql 'SELECT title, datetime FROM commits WHERE commit_conventional(title) = "feat"'"#,
                description: "Show title and datetime of commits with conventional title 'feat' using the only function commit_convetional()",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _plugin: &GitqlPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let curdir = engine.get_current_dir()?;
        // let path_to_use = expand_path_with(".", curdir, true);
        let query_string: String = call.req(0)?;

        let query_arguments = Arguments {
            repos: vec![curdir.to_string()],
            output_format: OutputFormat::Render,
            pagination: false,
            page_size: 10,
            analysis: false,
        };

        let mut reporter = diagnostic_reporter::DiagnosticReporter::default();
        let git_repos_result = validate_git_repositories(&query_arguments.repos);
        if git_repos_result.is_err() {
            reporter.report_diagnostic(
                &query_string,
                Diagnostic::error(git_repos_result.err().unwrap().as_str()),
            );
            return Err(LabeledError::new("Invalid repositories paths"));
        }

        let repos = git_repos_result.ok().unwrap();
        let schema = Schema {
            tables_fields_names: tables_fields_names().to_owned(),
            tables_fields_types: tables_fields_types().to_owned(),
        };

        let std_signatures = gitql_functions::gitql_std_signatures();
        let std_functions = gitql_functions::gitql_std_functions();

        let aggregation_signatures = aggregation_function_signatures();
        let aggregation_functions = aggregation_functions();

        let mut env = Environment::new(schema);
        env.with_standard_functions(std_signatures, std_functions);
        env.with_aggregation_functions(aggregation_signatures, aggregation_functions);

        Ok(execute_gitql_query(
            query_string,
            &query_arguments,
            &repos,
            &mut env,
            &mut reporter,
        ))

        // Ok(Value::nothing(call.head))
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    // This will automatically run the examples specified in your command and compare their actual
    // output against what was specified in the example. You can remove this test if the examples
    // can't be tested this way, but we recommend including it if possible.

    PluginTest::new("gitql", GitqlPlugin.into())?.test_command_examples(&Gitql)
}

fn main() {
    serve_plugin(&GitqlPlugin, MsgPackSerializer);
}

fn execute_gitql_query(
    query: String,
    query_arguments: &Arguments,
    repos: &[gix::Repository],
    env: &mut Environment,
    reporter: &mut DiagnosticReporter,
) -> Value {
    let front_start = std::time::Instant::now();
    let tokenizer_result = tokenizer::tokenize(query.clone());
    // eprintln!("1");
    if tokenizer_result.is_err() {
        let diagnostic = tokenizer_result.err().unwrap();
        reporter.report_diagnostic(&query, *diagnostic);
        return Value::test_string("tokenizer_result error".to_string());
    }

    // eprintln!("2");
    let tokens = tokenizer_result.ok().unwrap();
    if tokens.is_empty() {
        return Value::test_string("No tokens to parse".to_string());
    }

    // eprintln!("3");
    let parser_result = parser::parse_gql(tokens, env);
    if parser_result.is_err() {
        let diagnostic = parser_result.err().unwrap();
        reporter.report_diagnostic(&query, *diagnostic);
        return Value::test_string("parser_result error".to_string());
    }

    let query_node = parser_result.ok().unwrap();
    let front_duration = front_start.elapsed();

    let engine_start = std::time::Instant::now();
    let provider: Box<dyn DataProvider> = Box::new(GitDataProvider::new(repos.to_vec()));
    let evaluation_result = engine::evaluate(env, &provider, query_node);

    // eprintln!("4");

    // Report Runtime exceptions if they exists
    if evaluation_result.is_err() {
        reporter.report_diagnostic(
            &query,
            Diagnostic::exception(&evaluation_result.err().unwrap()),
        );
        return Value::test_string("evaluation_result error".to_string());
    }

    // eprintln!("5");

    // Render the result only if they are selected groups not any other statement
    let engine_result = evaluation_result.ok().unwrap();
    let output: Value = if let SelectedGroups(mut groups, hidden_selection) = engine_result {
        // eprintln!("6");
        // eprintln!("{:#?} -> {:#?}", groups.titles, hidden_selection);

        match query_arguments.output_format {
            OutputFormat::Render => {
                // render::render_objects(
                //     &mut groups,
                //     &hidden_selection,
                //     arguments.pagination,
                //     arguments.page_size,
                // );
                // eprintln!("6.1");

                nushell_render::render_objects(
                    &mut groups,
                    &hidden_selection,
                    query_arguments.pagination,
                    query_arguments.page_size,
                )
            }
            OutputFormat::JSON => {
                // eprintln!("6.2");
                if let Ok(json) = groups.as_json() {
                    Value::test_string(json)
                } else {
                    Value::test_string("No JSON data to show".to_string())
                }
            }
            OutputFormat::CSV => {
                // eprintln!("6.3");
                if let Ok(csv) = groups.as_csv() {
                    // eprintln!("6.3a");
                    Value::test_string(csv)
                } else {
                    // eprintln!("6.3b");
                    Value::test_string("No CSV data to show".to_string())
                }
            }
        }
    } else {
        // eprintln!("7");

        Value::test_string("Not a SelectedGroups result".to_string())
    };

    let engine_duration = engine_start.elapsed();

    if query_arguments.analysis {
        eprintln!("\n");
        eprintln!("Analysis:");
        eprintln!("Frontend : {:?}", front_duration);
        eprintln!("Engine   : {:?}", engine_duration);
        eprintln!("Total    : {:?}", (front_duration + engine_duration));
        eprintln!("\n");
    }

    output
}

fn validate_git_repositories(repositories: &Vec<String>) -> Result<Vec<gix::Repository>, String> {
    let mut git_repositories: Vec<gix::Repository> = vec![];
    for repository in repositories {
        let git_repository = gix::open(repository);
        if git_repository.is_err() {
            return Err(git_repository.err().unwrap().to_string());
        }
        git_repositories.push(git_repository.ok().unwrap());
    }
    Ok(git_repositories)
}
