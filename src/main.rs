//! Thin CLI over the testable library (M0 environment, M1-12 actor demo).
//!
//! Implemented now: `validate-config` and `simulate` (baseline lifetimes
//! plus B3 actor lifetimes with provenance + event logs, audited by
//! `analysis/validate_logs.py`). Planned (see README): `benchmark`,
//! `evolve`, `evaluate`, `intervene`. The CLI only wires library calls;
//! all behavior is testable without spawning a process.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use cra::config::load_and_validate;
use cra::run::{BaselineSel, EffectiveSeeds, run_simulation};

#[derive(Debug, Parser)]
#[command(
    name = "cra",
    version,
    about = "Learning When to Learn simulator (M0 environment, M1 nonplastic actor)"
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
    /// Run lifetimes into a fresh run directory with provenance
    /// and event logs. The environment stepping is real (M0-07 through
    /// M0-11 contracts); the nonplastic actor runs its inherited dynamics
    /// on every tick (M1-07) when the config carries an `[actor]` section.
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
        /// Baseline rung: random (B0), constant-0 | constant-1 (B1),
        /// actor (B3, needs an `[actor]` config such as
        /// configs/actor_no_learning.toml), oracle (O1, privileged
        /// reference).
        #[arg(long, default_value = "random")]
        baseline: String,
        /// Number of lifetimes to simulate (indices 0..lifetimes).
        #[arg(long, default_value_t = 1)]
        lifetimes: u64,
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
            baseline,
            lifetimes,
            out_dir,
        } => match load_and_validate(&config) {
            Ok(cfg) => {
                let seeds = EffectiveSeeds::from_config(&cfg).with_overrides(root_seed, outer_seed);
                let base = out_dir.unwrap_or_else(|| PathBuf::from("runs"));
                match BaselineSel::parse(&baseline)
                    .and_then(|sel| run_simulation(&cfg, &seeds, sel, lifetimes, &base))
                {
                    Ok(report) => {
                        println!("run dir: {}", report.dir.display());
                        println!(
                            "condition {}: {} lifetimes, {} commitments, {} outcomes, mean reward {:.4}",
                            report.condition_id,
                            report.lifetimes,
                            report.commitments,
                            report.outcomes,
                            report.mean_reward
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
