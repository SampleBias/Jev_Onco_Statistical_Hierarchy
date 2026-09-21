use clap::{Parser, Subcommand, ValueEnum};
use nexus_app::workflows::{self, AppError};
use nexus_core::errors::{ErrorCode, ErrorEnvelope};
use nexus_core::{JevResponse, Source, interpret, prepare};
use std::{io::Write, path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(
    name = "nexus",
    version,
    about = "Jev Onco Nexus — cancer-origin research from your terminal",
    after_help = "Start: nexus tui\nOffline: nexus demo --format text\nPipeline: nexus example | nexus validate -\nHelp: nexus <COMMAND> --help"
)]
struct Args {
    /// Output style; JSON is the default for scripts.
    #[arg(long, global = true, value_enum, default_value_t = Format::Json)]
    format: Format,
    /// Write to a NEW file instead of stdout. Existing files are protected.
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Json,
    Text,
}

#[derive(Clone, Copy, ValueEnum)]
enum SchemaKind {
    Case,
    JevRequest,
    JevResponse,
    Result,
    Error,
    Openapi,
}

#[derive(Subcommand)]
enum Command {
    /// Open the terminal workbench; defaults to a bundled synthetic case.
    Tui {
        #[arg(short, long)]
        case: Option<PathBuf>,
    },
    /// Print an editable synthetic case template; use --output to save it.
    Example,
    /// Check local setup without contacting Jev or displaying credentials.
    Doctor,
    /// List all outcomes in the development taxonomy.
    Taxonomy,
    /// Print a generated JSON Schema or the offline OpenAPI document.
    Schema {
        #[arg(value_enum)]
        kind: SchemaKind,
    },
    /// Validate case JSON. Use '-' to read stdin.
    Validate { case: PathBuf },
    /// Preview the exact Jev request without sending it. Use '-' to read stdin.
    Prepare { case: PathBuf },
    /// Run a labeled mock. Omit CASE for the bundled example.
    Demo { case: Option<PathBuf> },
    /// Interpret a provider fixture locally as an unverified replay.
    Replay { case: PathBuf, response: PathBuf },
    /// Send one synthetic case to Jev using TYPESAFE_API_KEY.
    Classify { case: PathBuf },
    /// Serve offline HTTP endpoints on loopback only.
    Serve {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },
}

async fn run(args: Args) -> Result<(), AppError> {
    let mut human = None;
    let value = match args.command {
        Command::Tui { case } => {
            if args.output.is_some() {
                return Err(ErrorEnvelope::new(ErrorCode::InvalidArguments).into());
            }
            return nexus_app::tui::run(case).await;
        }
        Command::Serve { port } => {
            if args.output.is_some() {
                return Err(ErrorEnvelope::new(ErrorCode::InvalidArguments).into());
            }
            let listener =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
            eprintln!("Offline research API: http://{}", listener.local_addr()?);
            axum::serve(listener, nexus_app::router())
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
            return Ok(());
        }
        Command::Example => serde_json::to_value(workflows::example_case())?,
        Command::Schema { kind } => {
            let name = match kind {
                SchemaKind::Case => "case.schema.json",
                SchemaKind::JevRequest => "jev-request.schema.json",
                SchemaKind::JevResponse => "jev-response.schema.json",
                SchemaKind::Result => "result.schema.json",
                SchemaKind::Error => "error.schema.json",
                SchemaKind::Openapi => "openapi.json",
            };
            nexus_app::contracts::documents()
                .remove(name)
                .expect("schema names are fixed")
        }
        Command::Doctor => {
            let value = workflows::doctor();
            human = Some(format!(
                "Jev Onco Nexus {}\nModel: {}\nOffline workflows: ready\nAPI key: {}\nLive data: synthetic only\nTUI terminal: {}\nProvider connectivity is not checked.\nCancer calibration: not validated",
                env!("CARGO_PKG_VERSION"),
                nexus_core::MODEL,
                if workflows::key_configured() {
                    "configured (hidden)"
                } else {
                    "missing — offline use is available"
                },
                value["tui_terminal_available"]
            ));
            value
        }
        Command::Taxonomy => {
            let options = nexus_core::taxonomy();
            human = Some(
                options
                    .iter()
                    .map(|(name, description)| format!("{name:<26} {description}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            serde_json::json!({"version": nexus_core::TAXONOMY_VERSION, "development_only": true, "options": options})
        }
        Command::Validate { case } => {
            let case = workflows::load_case(&case)?;
            human = Some(format!(
                "Valid case: {} ({} findings)",
                case.case_id,
                case.findings.len()
            ));
            serde_json::json!({"valid": true, "case_id": case.case_id, "findings": case.findings.len()})
        }
        Command::Prepare { case } => serde_json::to_value(prepare(&workflows::load_case(&case)?)?)?,
        Command::Demo { case } => {
            let case = match case {
                Some(path) => workflows::load_case(&path)?,
                None => workflows::example_case(),
            };
            let result = workflows::demo(&case)?;
            human = Some(workflows::result_text(&result));
            serde_json::to_value(result)?
        }
        Command::Replay { case, response } => {
            if case.as_os_str() == "-" && response.as_os_str() == "-" {
                return Err(ErrorEnvelope::new(ErrorCode::InvalidArguments).into());
            }
            let case = workflows::load_case(&case)?;
            let response: JevResponse = workflows::read_json(&response, 65_536)?;
            let result = interpret(&case, response, Source::Replay)
                .map_err(|_| ErrorEnvelope::new(ErrorCode::ProviderResponse))?;
            human = Some(workflows::result_text(&result));
            serde_json::to_value(result)?
        }
        Command::Classify { case } => {
            let result = workflows::classify(&workflows::load_case(&case)?).await?;
            human = Some(workflows::result_text(&result));
            serde_json::to_value(result)?
        }
    };
    let content = match args.format {
        Format::Json => workflows::pretty(&value)?,
        Format::Text => human.unwrap_or(workflows::pretty(&value)?),
    };
    if let Some(path) = args.output {
        workflows::save_new(&path, &content)?;
    } else {
        writeln!(std::io::stdout().lock(), "{content}")?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return ExitCode::SUCCESS;
            }
            let raw: Vec<_> = std::env::args_os().collect();
            let text = raw.iter().any(|arg| arg == "--format=text")
                || raw
                    .windows(2)
                    .any(|pair| pair[0] == "--format" && pair[1] == "text");
            print_error(
                ErrorEnvelope::new(ErrorCode::InvalidArguments),
                if text { Format::Text } else { Format::Json },
            );
            return ExitCode::from(2);
        }
    };
    let format = args.format;
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe)
            {
                return ExitCode::SUCCESS;
            }
            print_error(nexus_app::errors::envelope(&error), format);
            ExitCode::FAILURE
        }
    }
}

fn print_error(error: ErrorEnvelope, format: Format) {
    match format {
        Format::Json => eprintln!(
            "{}",
            serde_json::to_string(&error).expect("static error envelope serializes")
        ),
        Format::Text => eprintln!("error: {error}"),
    }
}
