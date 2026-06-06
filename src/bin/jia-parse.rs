use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "jia-parse", about = "Parse PDDL and .jia model files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse PDDL domain and/or problem files.
    Pddl(PddlArgs),
    /// Parse a .jia model file.
    Jia(JiaArgs),
}

#[derive(Parser)]
struct PddlArgs {
    /// Path to domain PDDL file.
    #[arg(short, long)]
    domain: Option<PathBuf>,

    /// Path to problem PDDL file.
    #[arg(short, long)]
    problem: Option<PathBuf>,

    /// Output parsed AST as JSON.
    #[arg(long)]
    json: bool,

    /// Print parse statistics.
    #[arg(long)]
    stats: bool,

    /// Validate-only mode.
    #[arg(long)]
    validate: bool,
}

#[derive(Parser)]
struct JiaArgs {
    /// Path to .jia/.jiacp model file.
    file: PathBuf,

    /// Output parsed AST as JSON.
    #[arg(long)]
    json: bool,

    /// Print parse statistics.
    #[arg(long)]
    stats: bool,

    /// Validate-only mode.
    #[arg(long)]
    validate: bool,
}

#[derive(Serialize)]
struct PddlJson<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<&'a jia_parse::ast::Domain>,
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<&'a jia_parse::ast::Problem>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Pddl(args) => run_pddl(args),
        Command::Jia(args) => run_jia(args),
    }
}

fn run_pddl(args: PddlArgs) -> Result<()> {
    if args.domain.is_none() && args.problem.is_none() {
        anyhow::bail!("No input files specified. Use --domain and/or --problem.");
    }

    let mut domain_result = None;
    let mut problem_result = None;

    if let Some(domain_path) = &args.domain {
        let input = std::fs::read_to_string(domain_path)
            .with_context(|| format!("reading {}", domain_path.display()))?;
        let start = std::time::Instant::now();
        let domain = jia_parse::pddl::parse_domain_str(&input)
            .map_err(|e| anyhow::anyhow!("{}: {}", domain_path.display(), e))?;
        let elapsed = start.elapsed();

        if args.stats {
            print_domain_stats(domain_path, &domain, elapsed);
        } else if args.validate {
            println!("OK  {}", domain_path.display());
        }

        domain_result = Some(domain);
    }

    if let Some(problem_path) = &args.problem {
        let input = std::fs::read_to_string(problem_path)
            .with_context(|| format!("reading {}", problem_path.display()))?;
        let start = std::time::Instant::now();
        let problem = jia_parse::pddl::parse_problem_str(&input)
            .map_err(|e| anyhow::anyhow!("{}: {}", problem_path.display(), e))?;
        let elapsed = start.elapsed();

        if args.stats {
            print_problem_stats(problem_path, &problem, elapsed);
        } else if args.validate {
            println!("OK  {}", problem_path.display());
        }

        problem_result = Some(problem);
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&PddlJson {
                domain: domain_result.as_ref(),
                problem: problem_result.as_ref(),
            })?
        );
    } else if !args.stats && !args.validate {
        if let Some(domain) = &domain_result {
            println!("Domain '{}' parsed successfully.", domain.name);
        }
        if let Some(problem) = &problem_result {
            println!("Problem '{}' parsed successfully.", problem.name);
        }
    }

    Ok(())
}

fn run_jia(args: JiaArgs) -> Result<()> {
    let input = std::fs::read_to_string(&args.file)
        .with_context(|| format!("reading {}", args.file.display()))?;
    let start = std::time::Instant::now();
    let model = jia_parse::jia::parse_model_str(&input)
        .map_err(|e| anyhow::anyhow!("{}: {}", args.file.display(), e))?;
    let elapsed = start.elapsed();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&model)?);
    } else if args.stats {
        println!("Model: {}", model.name);
        println!("  File: {}", args.file.display());
        println!("  Parse time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        println!("  Type: {:?}", model.model_type);
        println!("  Variables: {}", model.variables.len());
        println!("  Domains: {}", model.domains.len());
        println!("  Constraints: {}", model.constraints.len());
        println!(
            "  Has objective: {}",
            if model.objective.is_some() {
                "yes"
            } else {
                "no"
            }
        );
    } else if args.validate {
        println!("OK  {}", args.file.display());
    } else {
        println!("Model '{}' parsed successfully.", model.name);
    }

    Ok(())
}

fn print_domain_stats(
    domain_path: &std::path::Path,
    domain: &jia_parse::ast::Domain,
    elapsed: std::time::Duration,
) {
    println!("Domain: {}", domain.name);
    println!("  File: {}", domain_path.display());
    println!("  Parse time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Requirements: {:?}", domain.requirements);
    println!("  Type groups: {}", domain.types.len());
    println!("  Constants: {} group(s)", domain.constants.len());
    println!("  Predicates: {}", domain.predicates.len());
    println!("  Functions: {}", domain.functions.len());
    println!("  Actions: {}", domain.actions.len());
    println!("  Durative actions: {}", domain.durative_actions.len());
    println!("  Derived predicates: {}", domain.derived_predicates.len());
}

fn print_problem_stats(
    problem_path: &std::path::Path,
    problem: &jia_parse::ast::Problem,
    elapsed: std::time::Duration,
) {
    println!("Problem: {}", problem.name);
    println!("  File: {}", problem_path.display());
    println!("  Parse time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Domain: {}", problem.domain_name);
    println!("  Requirements: {:?}", problem.requirements);
    println!("  Object groups: {}", problem.objects.len());
    println!("  Init elements: {}", problem.init.len());
    println!(
        "  Has metric: {}",
        if problem.metric.is_some() {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  Has constraints: {}",
        if problem.constraints.is_some() {
            "yes"
        } else {
            "no"
        }
    );
}
