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
    print_header("LiteCode", "lightweight litellm coding harness");

    print_section("key benefits");
    println!(
        "  {} one command starts the harness loop",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} litellm-native routing for lower cost",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} clean output, low cognitive overhead",
        "•".truecolor(246, 178, 137)
    );

    print_section("brand theme");
    println!(
        "  {} dark surface, warm accent, understated text",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} sharp and calm engineering voice",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} tagline: fast coding harness on litellm",
        "•".truecolor(246, 178, 137)
    );

    print_architecture_diagram();
    print_flow_diagram();

    println!(
        "\n{} {}",
        "try".bold().bright_white(),
        "litecode run \"add retry middleware\" --quality balanced --budget cheap"
            .truecolor(148, 163, 184)
    );

    Ok(())
}

fn show_plan(task: &str) -> Result<()> {
    print_header("LiteCode", "plan mode");
    println!(
        "{} {}",
        "task".bold().bright_white(),
        task.truecolor(226, 232, 240)
    );

    print_section("execution plan");
    println!(
        "  {} understand task and repo context",
        "1.".truecolor(246, 178, 137)
    );
    println!(
        "  {} route model using quality and budget policy",
        "2.".truecolor(246, 178, 137)
    );
    println!(
        "  {} implement focused code changes",
        "3.".truecolor(246, 178, 137)
    );
    println!(
        "  {} run checks and summarize result",
        "4.".truecolor(246, 178, 137)
    );
    Ok(())
}

fn run_task(task: String, policy: RouterPolicy) -> Result<()> {
    let Some(config) = load_config()? else {
        bail!("not signed in. Run `litecode start` first.");
    };

    print_header("LiteCode", "run mode");
    println!(
        "{} {}",
        "task".bold().bright_white(),
        task.truecolor(226, 232, 240)
    );
    println!(
        "{} {}",
        "litellm".bold().bright_white(),
        config.base_url.truecolor(148, 163, 184)
    );
    println!(
        "{} quality={} budget={}",
        "policy".bold().bright_white(),
        format_quality(policy.quality).truecolor(226, 232, 240),
        format_budget(policy.budget).truecolor(226, 232, 240)
    );

    let model = select_model(&policy);
    println!(
        "{} {} ({}, est {})",
        "route".bold().bright_white(),
        model.name.truecolor(246, 178, 137),
        model.rationale.truecolor(148, 163, 184),
        model.estimated_cost.truecolor(148, 163, 184)
    );

    let loop_steps = [
        ("PLAN", "Build implementation checklist"),
        ("EXEC", "Generate patch and apply changes"),
        ("VERIFY", "Run targeted checks"),
        ("SUMMARIZE", "Return final result"),
    ];

    print_section("harness loop");
    for (phase, detail) in loop_steps {
        println!(
            "  {} {}  {}",
            "•".truecolor(246, 178, 137),
            phase.bold().bright_white(),
            detail.truecolor(203, 213, 225)
        );
    }

    println!(
        "\n{} {}",
        "outcome".bold().bright_white(),
        "ready to execute against your repo with low-cost routing defaults"
            .truecolor(226, 232, 240)
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
        "──────────────────────────────────────────────────────────────".truecolor(71, 85, 105)
    );
    println!(
        "{}  {}  {}",
        "✶".truecolor(246, 178, 137),
        title.bold().bright_white(),
        subtitle.truecolor(148, 163, 184)
    );
    println!(
        "{}",
        "──────────────────────────────────────────────────────────────".truecolor(71, 85, 105)
    );
}

fn print_section(name: &str) {
    println!();
    println!(
        "{} {}",
        "▍".truecolor(246, 178, 137),
        name.bold().truecolor(226, 232, 240)
    );
}

fn print_architecture_diagram() {
    print_section("architecture");
    println!("  developer");
    println!("    └─ litecode cli");
    println!("       └─ harness loop (plan → execute → verify → summarize)");
    println!("          └─ litellm auto router");
    println!("             ├─ fast-mini");
    println!("             ├─ coder-balanced");
    println!("             └─ reasoning-pro");
}

fn print_flow_diagram() {
    print_section("cost-optimized flow");
    println!("  task input");
    println!("    └─ policy (quality + budget)");
    println!("       └─ route model");
    println!("          └─ code loop");
    println!("             └─ patch output");
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
