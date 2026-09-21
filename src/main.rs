//! Thin CLI over the testable library (M0).
//!
//! Implemented now: `validate-config`, environment-scaffold `simulate`.
//! Planned (see README): `benchmark`, `evolve`, `evaluate`, `intervene`,
//! Python log analysis. The CLI only wires library calls; all behavior is
//! testable without spawning a process.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use cra::config::load_and_validate;
use cra::run::{EffectiveSeeds, create_run_dir};

#[derive(Debug, Parser)]
#[command(
    name = "cra",
    version,
    about = "Learning When to Learn simulator (M0 scaffold)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Parse and validate a TOML config; print the resolved profile summary.
    ValidateConfig {
        /// Path to the TOML config file.
        path: PathBuf,
    },
    /// M0 scaffold: validate the config, resolve seeds, and write a run
    /// directory with provenance. Full environment stepping arrives in
    /// M0-07+; this command never pretends to run an unimplemented actor.
    Simulate {
        /// Path to the TOML config file.
        #[arg(long)]
        config: PathBuf,
        /// Override the config's root seed (recorded as cli-sourced).
        /// `--seed` is accepted as a short alias for the spec's example
        /// invocation `simulate --config <file> --seed 1`.
        #[arg(long, visible_alias = "seed")]
        root_seed: Option<u64>,
        /// Override the config's outer seed (recorded as cli-sourced).
        #[arg(long)]
        outer_seed: Option<u64>,
        /// Base directory for run output (default: runs/).
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::ValidateConfig { path } => match load_and_validate(&path) {
            Ok(cfg) => {
                println!(
                    "OK: profile '{}' schema_version={} kind='{}' cues={} namespace='{}'",
                    cfg.profile_name,
                    cfg.schema_version,
                    cfg.environment.kind,
                    cfg.environment.cue_count,
                    cfg.seeds.namespace
                );
            }
            Err(e) => {
                eprintln!("ERROR: {e}");
                std::process::exit(1);
            }
        },
        Commands::Simulate {
            config,
            root_seed,
            outer_seed,
            out_dir,
        } => match load_and_validate(&config) {
            Ok(cfg) => {
                let seeds = EffectiveSeeds::from_config(&cfg).with_overrides(root_seed, outer_seed);
                let base = out_dir.unwrap_or_else(|| PathBuf::from("runs"));
                match create_run_dir(&cfg, &seeds, &base) {
                    Ok(dir) => {
                        println!("run dir: {}", dir.display());
                        println!(
                            "M0 scaffold: validated + wrote provenance; full environment stepping arrives in M0-07+."
                        );
                    }
                    Err(e) => {
                        eprintln!("ERROR: {e}");
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("ERROR: {e}");
                std::process::exit(1);
            }
        },
    }
}
