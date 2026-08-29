mod tui;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Password};
use owo_colors::OwoColorize;
use shipr_smart_routing::{Budget, Quality, resolve_routing_policy, select_model};
use std::fs;
use std::path::PathBuf;

pub(crate) const SHIP_BANNER: [&str; 10] = [
    "                  ╭─────╮",
    "            ╭─────┤ ▦ ▦ │",
    "      ╭─────┤ ▦ ▦ ├─────┤╭─────╮",
    "      │ ▦ ▦ ├─────┤ ▦ ▦ ││ ▦ ▦ │",
    "      ╰─────┴─────┴─────┴┴─────╯",
    "        ╲                           ╲___",
    "  ≋≋≋    ╲___   S H I P R R   _______/►",
    "≋≋≋≋≋≋≋≋≋≋╲_________________/≋≋≋≋≋≋≋≋≋≋",
    "       ≋≋≋≋         ≋≋≋≋         ≋≋≋≋",
    "          ship code. route smart. pay less.",
];

#[derive(Parser, Debug)]
#[command(
    name = "shipr",
    version,
    about = "Shiprr — minimal coding harness with smart low-cost routing",
    long_about = "A lightweight CLI harness for coding tasks.\nRuns a focused plan -> work -> verify loop and uses harness-level smart routing to reduce cost."
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
struct ShiprrConfig {
    base_url: String,
    api_key: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => launch_tui(),
        Some(Commands::Start) => start_setup(),
        Some(Commands::Run {
            task,
            quality,
            budget,
        }) => run_once(task, quality, budget),
        Some(Commands::Plan { task }) => show_plan(&task),
        Some(Commands::Preview) => preview_branding(),
    }
}

fn launch_tui() -> Result<()> {
    let config = ensure_config()?;
    tui::run(config.base_url)
}

fn ensure_config() -> Result<ShiprrConfig> {
    if let Some(config) = load_config()? {
        return Ok(config);
    }

    start_setup()?;
    load_config()?.context("failed to load config after setup")
}

fn start_setup() -> Result<()> {
    print_shiprr_banner();

    if let Some(config) = load_config()? {
        println!(
            "{} {}",
            "Already logged in:".truecolor(147, 197, 253),
            config.base_url.bright_white()
        );
        println!(
            "{} {}",
            "config:".truecolor(147, 197, 253),
            config_path().display().to_string().bright_white()
        );
        return Ok(());
    }

    println!();
    println!("{}", "Login to LiteLLM".bold().bright_white());

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

    let config = ShiprrConfig { base_url, api_key };
    validate_config(&config)?;
    save_config(&config)?;

    println!(
        "{}",
        "Login successful. Starting Shiprr…".truecolor(147, 197, 253)
    );
    Ok(())
}

fn run_once(
    task: String,
    quality_override: Option<Quality>,
    budget_override: Option<Budget>,
) -> Result<()> {
    let Some(config) = load_config()? else {
        bail!("not signed in. Run shipr start first.");
    };
    let routing = resolve_routing_policy(&task, quality_override, budget_override);
    let model = select_model(&routing.policy);

    println!();
    println!("{} {}", "Task".bold().bright_white(), task.bright_white());
    println!(
        "{} {} ({}, est {})",
        "Route".truecolor(147, 197, 253),
        model.name.bright_white(),
        model.rationale,
        model.estimated_cost
    );
    println!(
        "{} {} via {}",
        "Status".truecolor(147, 197, 253),
        "ready for execution".bright_white(),
        config.base_url
    );
    Ok(())
}

fn show_plan(task: &str) -> Result<()> {
    println!();
    println!("{} {}", "Plan".bold().bright_white(), task.bright_white());
    println!("  1. inspect repository context");
    println!("  2. select the cheapest capable route");
    println!("  3. apply focused changes");
    println!("  4. run checks and summarize");
    Ok(())
}

fn preview_branding() -> Result<()> {
    print_shiprr_banner();
    println!("{}", "minimal agentic coding CLI".bold().bright_white());
    println!(
        "{} persistent IDE-like terminal surface",
        "•".truecolor(96, 165, 250)
    );
    println!(
        "{} transient processing and durable responses",
        "•".truecolor(96, 165, 250)
    );
    println!(
        "{} smart routing at harness level",
        "•".truecolor(96, 165, 250)
    );
    Ok(())
}

fn print_shiprr_banner() {
    println!();
    for (index, line) in SHIP_BANNER.iter().enumerate() {
        let color = match index {
            0..=4 => (96, 165, 250),
            5..=6 => (59, 130, 246),
            7..=8 => (37, 99, 235),
            _ => (135, 145, 160),
        };
        println!("{}", line.bold().truecolor(color.0, color.1, color.2));
    }
    println!();
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".shipr").join("config.toml")
}

fn load_config() -> Result<Option<ShiprrConfig>> {
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

    let (Some(base_url), Some(api_key)) = (base_url, api_key) else {
        return Ok(None);
    };
    let config = ShiprrConfig { base_url, api_key };
    validate_config(&config)?;
    Ok(Some(config))
}

fn validate_config(config: &ShiprrConfig) -> Result<()> {
    if config.base_url.trim().is_empty() {
        bail!("base URL cannot be empty");
    }
    if config.api_key.trim().is_empty() {
        bail!("API key cannot be empty");
    }
    Ok(())
}

fn save_config(config: &ShiprrConfig) -> Result<()> {
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
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to secure config file {}", path.display()))?;
    }

    Ok(())
}
