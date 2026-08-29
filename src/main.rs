use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Password};
use owo_colors::OwoColorize;
use shipr_smart_routing::{Budget, Quality, resolve_routing_policy, select_model};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "shipr",
    version,
    about = "Shipr — minimal coding harness with smart low-cost routing",
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
struct ShiprConfig {
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
    print_header("shipr", "setup");

    if let Some(config) = load_config()? {
        println!(
            "{} already signed in to {}",
            "✓".bright_green(),
            config.base_url.bright_white()
        );
        print_kv("config", &config_path().display().to_string());
        return Ok(());
    }

    print_section("login");

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

    let config = ShiprConfig { base_url, api_key };
    validate_config(&config)?;
    save_config(&config)?;

    println!("{} signed in and saved config", "✓".bright_green());
    print_kv("next", "shipr run \"refactor retry flow\"");

    Ok(())
}

fn preview_branding() -> Result<()> {
    print_header("shipr", "minimal harness");

    print_section("pitch");
    println!("  {} only the agentic loop", "•".truecolor(246, 178, 137));
    println!(
        "  {} smart routing at harness level",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} tuned for lower cost than heavy coding agents",
        "•".truecolor(246, 178, 137)
    );

    print_section("theme");
    println!(
        "  {} dark terminal, low-noise layout",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "  {} single warm accent, compact typography",
        "•".truecolor(246, 178, 137)
    );

    print_architecture_diagram();
    print_flow_diagram();

    println!(
        "\n{} {}",
        "try".bold().bright_white(),
        "shipr run \"fix readme typo\"".truecolor(148, 163, 184)
    );
    println!(
        "{} {}",
        "try".bold().bright_white(),
        "shipr run \"investigate race condition in retries\"".truecolor(148, 163, 184)
    );

    Ok(())
}

fn show_plan(task: &str) -> Result<()> {
    print_header("shipr", "plan");
    print_kv("task", task);

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
        bail!("not signed in. Run `shipr start` first.");
    };

    let routing = resolve_routing_policy(&task, quality_override, budget_override);
    let policy = routing.policy;
    let model = select_model(&policy);

    print_header("shipr", "run");
    print_kv("task", &task);
    print_kv("litellm", &config.base_url);
    print_kv(
        "policy",
        &format!(
            "quality={} budget={}",
            format_quality(policy.quality),
            format_budget(policy.budget)
        ),
    );
    print_kv(
        "routing",
        &format!("{} ({})", routing.mode, routing.task_kind),
    );
    print_kv("why", &routing.reason);
    print_kv(
        "route",
        &format!(
            "{} ({}, est {})",
            model.name, model.rationale, model.estimated_cost
        ),
    );

    print_section("loop");
    for (phase, detail) in [
        ("PLAN", "build implementation checklist"),
        ("EXEC", "generate patch and apply changes"),
        ("VERIFY", "run targeted checks"),
        ("SUMMARIZE", "return final result"),
    ] {
        println!(
            "  {} {} {}",
            "•".truecolor(246, 178, 137),
            format!("[{}]", phase).bold().bright_white(),
            detail.truecolor(203, 213, 225)
        );
    }

    print_kv("status", "ready");

    Ok(())
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".shipr").join("config.toml")
}

fn load_config() -> Result<Option<ShiprConfig>> {
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

    let config = ShiprConfig { base_url, api_key };
    validate_config(&config)?;
    Ok(Some(config))
}

fn validate_config(config: &ShiprConfig) -> Result<()> {
    if config.base_url.trim().is_empty() {
        bail!("base URL cannot be empty");
    }
    if config.api_key.trim().is_empty() {
        bail!("API key cannot be empty");
    }
    Ok(())
}

fn save_config(config: &ShiprConfig) -> Result<()> {
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
        "──────────────────────────────────────────────────────────────".truecolor(51, 65, 85)
    );
    println!(
        "{}  {}  {}",
        "◉".truecolor(246, 178, 137),
        title.bold().bright_white(),
        subtitle.truecolor(148, 163, 184)
    );
    println!(
        "{}",
        "──────────────────────────────────────────────────────────────".truecolor(51, 65, 85)
    );
}

fn print_kv(key: &str, value: &str) {
    println!(
        "{} {}",
        format!("{key:>8}").truecolor(148, 163, 184),
        value.truecolor(226, 232, 240)
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
    println!("    └─ shipr");
    println!("       └─ agentic loop");
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
