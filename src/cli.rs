use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::{config::Config, generator, ir, parser};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Generate conservative C ABI wrappers from C++ headers"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Generate {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value_t = false)]
        dump_ir: bool,
    },
    Ir {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = IrFormat::Yaml)]
        format: IrFormat,
    },
    Check {
        #[arg(long)]
        config: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum IrFormat {
    Yaml,
    Json,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate { config, dump_ir } => {
            let config = Config::load(config)?;
            generator::generate_all(&config, dump_ir)?;
        }
        Command::Ir {
            config,
            output,
            format,
        } => {
            let config = Config::load(config)?;
            let parsed = parser::parse(&config)?;
            let ir = ir::normalize(&config, &parsed)?;
            match (output, format) {
                (Some(path), IrFormat::Yaml) => generator::write_ir(&path, &ir)?,
                (Some(path), IrFormat::Json) => {
                    std::fs::write(path, serde_json::to_string_pretty(&ir)?)?
                }
                (None, IrFormat::Yaml) => print!("{}", serde_yaml::to_string(&ir)?),
                (None, IrFormat::Json) => print!("{}", serde_json::to_string_pretty(&ir)?),
            }
        }
        Command::Check { config } => {
            let config = Config::load(config)?;
            let parsed = parser::parse(&config)?;
            let ir = ir::normalize(&config, &parsed)?;
            println!(
                "ok: {} headers, {} classes, {} functions, {} enums, {} abi functions",
                parsed.headers.len(),
                parsed.classes.len(),
                parsed.functions.len(),
                parsed.enums.len(),
                ir.functions.len()
            );
        }
    }
    Ok(())
}
