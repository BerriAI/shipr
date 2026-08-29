use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Password};
use owo_colors::OwoColorize;
use routr_smart_routing::{Budget, Quality, resolve_routing_policy, select_model};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "routr",
    version,
    about = "Routr — minimal coding harness with smart low-cost routing",
    long_about = "A lightweight CLI harness for coding tasks.\nRuns a tiny plan -> execute -> verify -> summarize loop and uses harness-level smart routing to reduce cost."
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
        #[arg(long, value_enum)]
        quality: Option<Quality>,
        #[arg(long, value_enum)]
        budget: Option<Budget>,
    },
    Plan {
        #[arg(help = "Task to plan")]
        task: String,
    },
    Preview,
}

#[derive(Debug)]
struct RoutrConfig {
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
        } => run_task(task, quality, budget),
        Commands::Plan { task } => show_plan(&task),
        Commands::Preview => preview_branding(),
    }
}

fn start_setup() -> Result<()> {
    print_header("Routr", "lightweight harness setup");

    if let Some(config) = load_config()? {
        println!(
            "{} already signed in to {}",
            "✓".bright_green(),
            config.base_url.bright_white()
        );
        println!(
            "{} {}",
            "config".bold().bright_white(),
            config_path().display().to_string().truecolor(148, 163, 184)
        );
        return Ok(());
    }

    print_section("sign in to litellm");

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

    let config = RoutrConfig { base_url, api_key };
    validate_config(&config)?;
    save_config(&config)?;

    println!("{} signed in and saved config", "✓".bright_green());
    println!(
        "{} {}",
        "next".bold().bright_white(),
        "routr run \"refactor retry flow\"".truecolor(148, 163, 184)
    );

    Ok(())
}

fn preview_branding() -> Result<()> {
    print_header("Routr", "minimal loop. smarter routing. lower cost.");

    print_section("core pitch");
    println!(
        "  {} lightweight by default: just the agentic loop",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} harness-level smart router for cheaper model selection",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} built to be materially cheaper than heavy coding agents",
        "•".truecolor(246, 178, 137)
    );

    print_section("aesthetic");
    println!(
        "  {} dark terminal, low-noise layout, warm accent",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} compact sections, understated typography",
        "•".truecolor(246, 178, 137)
    );

    print_architecture_diagram();
    print_flow_diagram();

    println!(
        "\n{} {}",
        "try".bold().bright_white(),
        "routr run \"fix readme typo\"".truecolor(148, 163, 184)
    );
    println!(
        "{} {}",
        "try".bold().bright_white(),
        "routr run \"investigate race condition in retries\"".truecolor(148, 163, 184)
    );

    Ok(())
}

fn show_plan(task: &str) -> Result<()> {
    print_header("Routr", "plan mode");
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
        "  {} smart-route by complexity and cost policy",
        "2.".truecolor(246, 178, 137)
    );
    println!(
        "  {} apply focused code changes",
        "3.".truecolor(246, 178, 137)
    );
    println!(
        "  {} verify output and summarize",
        "4.".truecolor(246, 178, 137)
    );

    Ok(())
}

fn run_task(
    task: String,
    quality_override: Option<Quality>,
    budget_override: Option<Budget>,
) -> Result<()> {
    let Some(config) = load_config()? else {
        bail!("not signed in. Run `routr start` first.");
    };

    let routing = resolve_routing_policy(&task, quality_override, budget_override);
    let policy = routing.policy;
    let model = select_model(&policy);

    print_header("Routr", "run mode");
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
    println!(
        "{} {} ({})",
        "routing".bold().bright_white(),
        routing.mode.truecolor(246, 178, 137),
        routing.task_kind.truecolor(148, 163, 184)
    );
    println!(
        "{} {}",
        "why".bold().bright_white(),
        routing.reason.truecolor(148, 163, 184)
    );
    println!(
        "{} {} ({}, est {})",
        "route".bold().bright_white(),
        model.name.truecolor(246, 178, 137),
        model.rationale.truecolor(148, 163, 184),
        model.estimated_cost.truecolor(148, 163, 184)
    );

    print_section("agentic loop");
    for (phase, detail) in [
        ("PLAN", "build implementation checklist"),
        ("EXEC", "generate patch and apply changes"),
        ("VERIFY", "run targeted checks"),
        ("SUMMARIZE", "return final result"),
    ] {
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
        "minimal loop with cost-aware smart routing ready".truecolor(226, 232, 240)
    );

    Ok(())
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".routr").join("config.toml")
}

fn load_config() -> Result<Option<RoutrConfig>> {
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

    let config = RoutrConfig { base_url, api_key };
    validate_config(&config)?;
    Ok(Some(config))
}

fn validate_config(config: &RoutrConfig) -> Result<()> {
    if config.base_url.trim().is_empty() {
        bail!("base URL cannot be empty");
    }
    if config.api_key.trim().is_empty() {
        bail!("API key cannot be empty");
    }
    Ok(())
}

fn save_config(config: &RoutrConfig) -> Result<()> {
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
    println!("    └─ routr");
    println!("       └─ minimal agentic loop");
    println!("          └─ smart routing crate");
    println!("             └─ litellm auto router");
}

fn print_flow_diagram() {
    print_section("cost flow");
    println!("  task intent");
    println!("    └─ infer complexity");
    println!("       └─ choose quality and budget");
    println!("          └─ pick cheapest viable model");
    println!("             └─ execute loop and return patch");
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
