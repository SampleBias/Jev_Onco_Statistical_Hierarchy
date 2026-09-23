use clap::{Parser, Subcommand, ValueEnum};
use josh_app::workflows::{self, AppError};
use josh_core::errors::{ErrorCode, ErrorEnvelope};
use josh_core::{JevResponse, Source, interpret, prepare};
use std::{io::Write, path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(
    name = "josh",
    version,
    about = "Jev Onco Statistical Hierarchy (JOSH) — cancer-origin research from your terminal",
    after_help = "Start: josh tui\nOffline: josh demo --format text\nPipeline: josh example | josh validate -\nHelp: josh <COMMAND> --help"
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
    ImportReport,
    Splits,
    Labels,
    Guidance,
    Sample,
    Dataset,
    ExpressionRecord,
}

#[derive(Clone, Copy, ValueEnum)]
enum InputFormat {
    Json,
    Jsonl,
    Csv,
    Tsv,
}

#[derive(Subcommand)]
enum Command {
    /// Molecular dataset import, QC, exploration and reproducibility.
    Dataset {
        #[command(subcommand)]
        command: Box<josh_app::data::DatasetCommand>,
    },
    /// Open the sample/data workbench; legacy cases remain available explicitly.
    Tui {
        #[arg(short, long, conflicts_with = "batch")]
        case: Option<PathBuf>,
        /// Open a verified import bundle and browse cases with [ and ].
        #[arg(long, conflicts_with = "case")]
        batch: Option<PathBuf>,
        /// Open a molecular dataset bundle.
        #[arg(long, conflicts_with_all = ["case", "batch", "legacy"])]
        dataset: Option<PathBuf>,
        /// Open the original case and clinical-review interface.
        #[arg(long)]
        legacy: bool,
    },
    /// Print an editable synthetic case template; use --output to save it.
    Example {
        /// Include schema 3 clinical context and an invented investigation timeline.
        #[arg(long)]
        clinical: bool,
    },
    /// Evaluate the local, versioned NICE CG104 review rules; no provider request.
    Guidance { case: PathBuf },
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
    /// Import/migrate evidence locally; never sends records to Jev. Exit 3 means rejected records.
    #[command(
        after_help = "Example: josh import fixtures/import/synthetic-findings.csv --input-format csv --source-id synthetic-v1 --out-dir results/run-001\nJSONL contains one canonical case per line. CSV/TSV is one finding per row.\nRequired columns: case_id,data_class,finding_id,kind,name,value\nSee docs/data/IMPORT_GUIDE.md for optional columns, labels and partitions.\nLimits: 64 MiB source, 50,000 records, 2,000 cases. Output directory must be new; parent must exist."
    )]
    Import {
        /// Input file or '-' for stdin.
        input: PathBuf,
        #[arg(long, value_enum)]
        input_format: InputFormat,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long, default_value = "development-v1")]
        split_seed: String,
        #[arg(long)]
        holdout_institution: Option<String>,
    },
    /// Verify an import bundle's sidecars and show its quality report.
    Batch { directory: PathBuf },
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

async fn run(args: Args) -> Result<bool, AppError> {
    let mut human = None;
    let mut raw_output = None;
    let mut rejected_records = false;
    let value = match args.command {
        Command::Dataset { command } => {
            if args.output.is_some()
                && matches!(
                    &*command,
                    josh_app::data::DatasetCommand::Import { .. }
                        | josh_app::data::DatasetCommand::MigrateCase { .. }
                )
            {
                return Err(ErrorEnvelope::new(ErrorCode::InvalidArguments).into());
            }
            let result = josh_app::data::execute(*command)?;
            human = Some(result.human);
            raw_output = result.raw;
            rejected_records = result.blocked;
            result.value
        }
        Command::Tui {
            case,
            batch,
            dataset,
            legacy,
        } => {
            if args.output.is_some() {
                return Err(ErrorEnvelope::new(ErrorCode::InvalidArguments).into());
            }
            return if legacy || case.is_some() || batch.is_some() {
                josh_app::tui::run(case, batch).await
            } else {
                josh_app::workbench::run(dataset).await
            }
            .map(|()| false);
        }
        Command::Serve { port } => {
            if args.output.is_some() {
                return Err(ErrorEnvelope::new(ErrorCode::InvalidArguments).into());
            }
            let listener =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
            eprintln!("Offline research API: http://{}", listener.local_addr()?);
            axum::serve(listener, josh_app::router())
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
            return Ok(false);
        }
        Command::Example { clinical } => serde_json::to_value(if clinical {
            workflows::clinical_example_case()
        } else {
            workflows::example_case()
        })?,
        Command::Guidance { case } => {
            let report = josh_core::guidance::evaluate(&workflows::load_case(&case)?)?;
            human = Some(workflows::guidance_text(&report));
            serde_json::to_value(report)?
        }
        Command::Schema { kind } => {
            let name = match kind {
                SchemaKind::Case => "case.schema.json",
                SchemaKind::JevRequest => "jev-request.schema.json",
                SchemaKind::JevResponse => "jev-response.schema.json",
                SchemaKind::Result => "result.schema.json",
                SchemaKind::Error => "error.schema.json",
                SchemaKind::Openapi => "openapi.json",
                SchemaKind::ImportReport => "import-report.schema.json",
                SchemaKind::Splits => "splits.schema.json",
                SchemaKind::Labels => "labels.schema.json",
                SchemaKind::Guidance => "guidance.schema.json",
                SchemaKind::Sample => "sample.schema.json",
                SchemaKind::Dataset => "dataset.schema.json",
                SchemaKind::ExpressionRecord => "expression-record.schema.json",
            };
            josh_app::contracts::documents()
                .remove(name)
                .expect("schema names are fixed")
        }
        Command::Doctor => {
            let value = workflows::doctor();
            human = Some(format!(
                "Jev Onco Statistical Hierarchy (JOSH) {}\nModel: {}\nOffline workflows: ready\nAPI key: {}\nLive data: synthetic only\nTUI terminal: {}\nProvider connectivity is not checked.\nCancer calibration: not validated",
                env!("CARGO_PKG_VERSION"),
                josh_core::MODEL,
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
            let options = josh_core::taxonomy();
            human = Some(
                options
                    .iter()
                    .map(|(name, description)| format!("{name:<26} {description}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            serde_json::json!({"version": josh_core::TAXONOMY_VERSION, "development_only": true, "options": options})
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
        Command::Import {
            input,
            input_format,
            source_id,
            out_dir,
            split_seed,
            holdout_institution,
        } => {
            // A report is always stored inside the bundle. Avoid a second export failing
            // after a successful import, which would obscure the partial-import exit status.
            if args.output.is_some() {
                return Err(ErrorEnvelope::new(ErrorCode::InvalidArguments).into());
            }
            let format = match input_format {
                InputFormat::Json => josh_ingest::InputFormat::Json,
                InputFormat::Jsonl => josh_ingest::InputFormat::Jsonl,
                InputFormat::Csv => josh_ingest::InputFormat::Csv,
                InputFormat::Tsv => josh_ingest::InputFormat::Tsv,
            };
            let report = workflows::import_cases(
                &input,
                &josh_ingest::Options {
                    format,
                    source_id,
                    split_seed,
                    holdout_institution,
                },
                &out_dir,
            )?;
            rejected_records = report.rejected_records > 0;
            human = Some(format!(
                "{}\n\nBundle directory: {}\nOpen this directory with josh tui --batch.",
                workflows::import_report_text(&report),
                workflows::display_text(&out_dir.display().to_string())
            ));
            serde_json::to_value(report)?
        }
        Command::Batch { directory } => {
            let report = josh_ingest::bundle::read_report(&directory)?;
            // Inspecting a complete partial/rejected report succeeds; import exit 3 records the failure.
            human = Some(workflows::import_report_text(&report));
            serde_json::to_value(report)?
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
    let content = if let Some(raw) = raw_output {
        raw
    } else {
        match args.format {
            Format::Json => workflows::pretty(&value)?,
            Format::Text => human.unwrap_or(workflows::pretty(&value)?),
        }
    };
    if let Some(path) = args.output {
        workflows::save_new(&path, &content)?;
    } else {
        writeln!(std::io::stdout().lock(), "{content}")?;
    }
    Ok(rejected_records)
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
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(3),
        Err(error) => {
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe)
            {
                return ExitCode::SUCCESS;
            }
            print_error(josh_app::errors::envelope(&error), format);
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
