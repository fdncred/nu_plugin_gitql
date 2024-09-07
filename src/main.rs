use crate::gitql_schema::tables_fields_names;
use crate::gitql_schema::tables_fields_types;
use nu_plugin::{serve_plugin, MsgPackSerializer, Plugin, PluginCommand};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, SyntaxShape, Value};
// use atty::Stream;
use gitql_cli::arguments;
use gitql_cli::arguments::Arguments;
use gitql_cli::arguments::Command;
use gitql_cli::arguments::OutputFormat;
use gitql_cli::diagnostic_reporter;
use gitql_cli::diagnostic_reporter::DiagnosticReporter;
use gitql_cli::render;
use gitql_core::environment::Environment;
use gitql_core::schema::Schema;
use gitql_data_provider::GitDataProvider;
use gitql_engine::data_provider::DataProvider;
use gitql_engine::engine;
use gitql_engine::engine::EvaluationResult::SelectedGroups;
use gitql_parser::diagnostic::Diagnostic;
use gitql_parser::parser;
use gitql_parser::tokenizer;
use gitql_std::aggregation::aggregation_function_signatures;
use gitql_std::aggregation::aggregation_functions;

mod gitql_data_provider;
mod gitql_functions;
mod gitql_schema;

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
            // .required(
            //     "name",
            //     SyntaxShape::String,
            //     "(FIXME) A demo parameter - your name",
            // )
            .rest("query", SyntaxShape::String, "gitql query string")
            // .switch("shout", "(FIXME) Yell it instead", None)
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "(FIXME) help text for gitql"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "gitql Ellie",
                description: "Say hello to Ellie",
                result: Some(Value::test_string("Hello, Ellie. How are you today?")),
            },
            Example {
                example: "gitql --shout Ellie",
                description: "Shout hello to Ellie",
                result: Some(Value::test_string("HELLO, ELLIE. HOW ARE YOU TODAY?")),
            },
        ]
    }

    fn run(
        &self,
        _plugin: &GitqlPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        // let name: String = call.req(0)?;
        // let mut greeting = format!("Hello, {name}. How are you today?");
        // if call.has_flag("shout")? {
        //     greeting = greeting.to_uppercase();
        // }
        // Ok(Value::string(greeting, call.head))

        let args: Vec<String> = call.rest(0)?;
        eprintln!("args: {:?}", args);

        let arguments = arguments::parse_arguments(&args);
        eprintln!("arguments: {:?}", arguments);

        let (query, query_arguments) = if let Command::QueryMode(q, qa) = arguments {
            eprintln!("query: {:?} query_arguments: {:?}", q, qa);
            (q, qa)
        } else {
            return Err(LabeledError::new("Invalid command query mode arguments"));
        };

        let mut reporter = diagnostic_reporter::DiagnosticReporter::default();
        let git_repos_result = validate_git_repositories(&query_arguments.repos);
        if git_repos_result.is_err() {
            reporter.report_diagnostic(
                &query,
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

        execute_gitql_query(query, &query_arguments, &repos, &mut env, &mut reporter);

        Ok(Value::nothing(call.head))
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
    arguments: &Arguments,
    repos: &[gix::Repository],
    env: &mut Environment,
    reporter: &mut DiagnosticReporter,
) {
    let front_start = std::time::Instant::now();
    let tokenizer_result = tokenizer::tokenize(query.clone());
    if tokenizer_result.is_err() {
        let diagnostic = tokenizer_result.err().unwrap();
        reporter.report_diagnostic(&query, *diagnostic);
        return;
    }

    let tokens = tokenizer_result.ok().unwrap();
    if tokens.is_empty() {
        return;
    }

    let parser_result = parser::parse_gql(tokens, env);
    if parser_result.is_err() {
        let diagnostic = parser_result.err().unwrap();
        reporter.report_diagnostic(&query, *diagnostic);
        return;
    }

    let query_node = parser_result.ok().unwrap();
    let front_duration = front_start.elapsed();

    let engine_start = std::time::Instant::now();
    let provider: Box<dyn DataProvider> = Box::new(GitDataProvider::new(repos.to_vec()));
    let evaluation_result = engine::evaluate(env, &provider, query_node);

    // Report Runtime exceptions if they exists
    if evaluation_result.is_err() {
        reporter.report_diagnostic(
            &query,
            Diagnostic::exception(&evaluation_result.err().unwrap()),
        );
        return;
    }

    // Render the result only if they are selected groups not any other statement
    let engine_result = evaluation_result.ok().unwrap();
    if let SelectedGroups(mut groups, hidden_selection) = engine_result {
        match arguments.output_format {
            OutputFormat::Render => {
                render::render_objects(
                    &mut groups,
                    &hidden_selection,
                    arguments.pagination,
                    arguments.page_size,
                );
            }
            OutputFormat::JSON => {
                if let Ok(json) = groups.as_json() {
                    println!("{}", json);
                }
            }
            OutputFormat::CSV => {
                if let Ok(csv) = groups.as_csv() {
                    println!("{}", csv);
                }
            }
        }
    }

    let engine_duration = engine_start.elapsed();

    if arguments.analysis {
        println!("\n");
        println!("Analysis:");
        println!("Frontend : {:?}", front_duration);
        println!("Engine   : {:?}", engine_duration);
        println!("Total    : {:?}", (front_duration + engine_duration));
        println!("\n");
    }
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
