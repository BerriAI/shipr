use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use dialoguer::{Input, Password};
use owo_colors::OwoColorize;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "litecode",
    version,
    about = "LiteCode CLI — fast coding harness for LiteLLM",
    long_about = "A super lightweight CLI harness for coding tasks.\nRuns a simple plan -> execute -> verify -> summarize loop and optimizes for cost with LiteLLM Auto Router."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start,
    Run {
        #[arg(help = "Coding task to execute")]
        task: String,
        #[arg(long, value_enum, default_value_t = Quality::Balanced)]
        quality: Quality,
        #[arg(long, value_enum, default_value_t = Budget::Cheap)]
        budget: Budget,
    },
    Plan {
        #[arg(help = "Task to plan")]
        task: String,
    },
    Preview,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Quality {
    Fast,
    Balanced,
    High,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Budget {
    Cheapest,
    Cheap,
    Flexible,
}

#[derive(Debug)]
struct RouterPolicy {
    quality: Quality,
    budget: Budget,
}

#[derive(Debug)]
struct ModelChoice {
    name: &'static str,
    rationale: &'static str,
    estimated_cost: &'static str,
}

#[derive(Debug)]
struct LiteCodeConfig {
    base_url: String,
    api_key: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Start) {
        Commands::Start => start_setup(),
        Commands::Run {
            task,
            quality,
            budget,
        } => run_task(task, RouterPolicy { quality, budget }),
        Commands::Plan { task } => show_plan(&task),
        Commands::Preview => preview_branding(),
    }
}

fn start_setup() -> Result<()> {
    print_header("LiteCode Start", "Fast coding harness setup");

    if let Some(config) = load_config()? {
        println!(
            "{} already signed in to {}",
            "✓".bright_green(),
            config.base_url.bright_white()
        );
        println!(
            "{} {}",
            "Config:".bold().white(),
            config_path().display().to_string().bright_black()
        );
        return Ok(());
    }

    println!("{}", "Sign in to LiteLLM".bold().cyan());

    let base_url: String = Input::new()
        .with_prompt("LiteLLM base URL")
        .default("http://localhost:4000".to_string())
        .interact_text()
        .context("failed to read base URL input")?;

    let api_key = Password::new()
        .with_prompt("LiteLLM API key")
        .allow_empty_password(false)
        .interact()
        .context("failed to read API key input")?;

    let config = LiteCodeConfig { base_url, api_key };
    validate_config(&config)?;
    save_config(&config)?;

    println!("{} signed in and saved config", "✓".bright_green(),);
    println!(
        "{} {}",
        "Config:".bold().white(),
        config_path().display().to_string().bright_black()
    );
    println!(
        "{} {}",
        "Next:".bold().white(),
        "litecode run \"fix provider fallback\" --quality balanced --budget cheap".bright_black()
    );

    Ok(())
}

fn preview_branding() -> Result<()> {
    print_header("LiteCode CLI", "Code fast. Route smart. Spend less.");

    println!("{}", "\nKey Benefits".bold().cyan());
    println!(
        "  {} Simple UX: one command starts the harness loop",
        "●".bright_blue()
    );
    println!(
        "  {} LiteLLM-native routing for lower cost",
        "●".bright_blue()
    );
    println!(
        "  {} Devtool feel: clean output, tiny mental overhead",
        "●".bright_blue()
    );

    println!("{}", "\nBrand Theme".bold().cyan());
    println!(
        "  {} Dark terminal, cyan accents, precise status lines",
        "●".bright_green()
    );
    println!(
        "  {} Voice: sharp, calm, engineering-first",
        "●".bright_green()
    );
    println!(
        "  {} Tagline: \"Fast coding harness on LiteLLM\"",
        "●".bright_green()
    );

    print_architecture_diagram();
    print_flow_diagram();

    println!(
        "\n{} {}",
        "Try:".bold().white(),
        "litecode run \"add retry middleware\" --quality balanced --budget cheap".bright_black()
    );

    Ok(())
}

fn show_plan(task: &str) -> Result<()> {
    print_header("LiteCode Plan", "Minimal harness loop for coding tasks");
    println!("{} {}", "Task:".bold().white(), task.bright_white());

    println!("\n{}", "Execution Plan".bold().cyan());
    println!(
        "  1) {} Understand task + repo context",
        "PLAN".bright_blue()
    );
    println!("  2) {} Route model via policy", "ROUTE".bright_blue());
    println!("  3) {} Implement focused changes", "BUILD".bright_blue());
    println!("  4) {} Run checks + summarize", "VERIFY".bright_blue());
    Ok(())
}

fn run_task(task: String, policy: RouterPolicy) -> Result<()> {
    let Some(config) = load_config()? else {
        bail!("not signed in. Run `litecode start` first.");
    };

    print_header("LiteCode Run", "Lightweight coding harness");
    println!("{} {}", "Task:".bold().white(), task.bright_white());
    println!(
        "{} {}",
        "LiteLLM:".bold().white(),
        config.base_url.bright_black()
    );
    println!(
        "{} quality={} budget={}",
        "Policy:".bold().white(),
        format_quality(policy.quality).bright_cyan(),
        format_budget(policy.budget).bright_green()
    );

    let model = select_model(&policy);
    println!(
        "{} {} ({}, est {})",
        "Route:".bold().white(),
        model.name.bright_yellow(),
        model.rationale.bright_black(),
        model.estimated_cost.bright_black()
    );

    let loop_steps = [
        ("PLAN", "Build implementation checklist"),
        ("EXEC", "Generate patch and apply changes"),
        ("VERIFY", "Run targeted checks"),
        ("SUMMARIZE", "Return final result"),
    ];

    println!("\n{}", "Harness Loop".bold().cyan());
    for (phase, detail) in loop_steps {
        println!("  {} {:<9} {}", "▶".bright_blue(), phase.bold(), detail);
    }

    println!(
        "\n{} {}",
        "Outcome:".bold().white(),
        "Ready to execute against your repo with low-cost routing defaults".bright_white()
    );

    Ok(())
}

fn select_model(policy: &RouterPolicy) -> ModelChoice {
    match (policy.quality, policy.budget) {
        (Quality::Fast, Budget::Cheapest) => ModelChoice {
            name: "fast-mini",
            rationale: "max speed / lowest cost",
            estimated_cost: "$",
        },
        (Quality::High, Budget::Flexible) => ModelChoice {
            name: "reasoning-pro",
            rationale: "deeper coding reasoning",
            estimated_cost: "$$$",
        },
        _ => ModelChoice {
            name: "coder-balanced",
            rationale: "best quality/cost mix",
            estimated_cost: "$$",
        },
    }
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".litecode").join("config.toml")
}

fn load_config() -> Result<Option<LiteCodeConfig>> {
    let path = config_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;

    let mut base_url = None;
    let mut api_key = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let cleaned = value.trim().trim_matches('"').to_string();
            match key.trim() {
                "base_url" => base_url = Some(cleaned),
                "api_key" => api_key = Some(cleaned),
                _ => {}
            }
        }
    }

    let Some(base_url) = base_url else {
        return Ok(None);
    };
    let Some(api_key) = api_key else {
        return Ok(None);
    };

    let config = LiteCodeConfig { base_url, api_key };
    validate_config(&config)?;
    Ok(Some(config))
}

fn validate_config(config: &LiteCodeConfig) -> Result<()> {
    if config.base_url.trim().is_empty() {
        bail!("base URL cannot be empty");
    }
    if config.api_key.trim().is_empty() {
        bail!("API key cannot be empty");
    }
    Ok(())
}

fn save_config(config: &LiteCodeConfig) -> Result<()> {
    let path = config_path();
    let parent = path.parent().context("missing config parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create config directory {}", parent.display()))?;

    let content = format!(
        "base_url = \"{}\"\napi_key = \"{}\"\n",
        config.base_url, config.api_key
    );
    fs::write(&path, content)
        .with_context(|| format!("failed to write config file {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).with_context(|| {
            format!(
                "failed to set secure permissions on config file {}",
                path.display()
            )
        })?;
    }

    Ok(())
}

fn print_header(title: &str, subtitle: &str) {
    println!();
    println!(
        "{}",
        "╭──────────────────────────────────────────────────────────────╮".bright_blue()
    );
    println!(
        "{} {} {}",
        "│".bright_blue(),
        "LiteCode CLI".bold().bright_cyan(),
        "│".bright_blue()
    );
    println!(
        "{} {} {}",
        "│".bright_blue(),
        subtitle.bright_black(),
        "│".bright_blue()
    );
    println!(
        "{}",
        "╰──────────────────────────────────────────────────────────────╯".bright_blue()
    );
    println!("{}", title.bold().bright_white());
}

fn print_architecture_diagram() {
    println!("\n{}", "Architecture".bold().cyan());
    println!("  Developer");
    println!("      │");
    println!("      ▼");
    println!("  LiteCode CLI");
    println!("      │");
    println!("      ▼");
    println!("  Harness Loop (plan → execute → verify → summarize)");
    println!("      │");
    println!("      ▼");
    println!("  LiteLLM Auto Router");
    println!("    ├─ fast-mini");
    println!("    ├─ coder-balanced");
    println!("    └─ reasoning-pro");
}

fn print_flow_diagram() {
    println!("\n{}", "Cost-Optimized Flow".bold().cyan());
    println!("  task input → policy (quality+budget) → route model → code loop → patch output");
}

fn format_quality(quality: Quality) -> &'static str {
    match quality {
        Quality::Fast => "fast",
        Quality::Balanced => "balanced",
        Quality::High => "high",
    }
}

fn format_budget(budget: Budget) -> &'static str {
    match budget {
        Budget::Cheapest => "cheapest",
        Budget::Cheap => "cheap",
        Budget::Flexible => "flexible",
    }
}
