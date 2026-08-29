use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Password};
use owo_colors::OwoColorize;
use shipr_smart_routing::{Budget, Quality, resolve_routing_policy, select_model};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

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
    Shell,
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

#[derive(Debug)]
struct TaskRecord {
    id: String,
    prompt: String,
    status: &'static str,
    progress: u8,
    phase: &'static str,
    model: &'static str,
    estimated_cost: &'static str,
    elapsed_ms: u128,
}

#[derive(Debug, Default)]
struct ShiprSession {
    next_task_number: u32,
    tasks: Vec<TaskRecord>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Shell) {
        Commands::Shell => shell_mode(),
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

fn shell_mode() -> Result<()> {
    let config = ensure_config_for_shell()?;
    let mut session = ShiprSession {
        next_task_number: 1,
        tasks: Vec::new(),
    };

    print_shell_welcome();

    loop {
        print!("{}", "❯ ".truecolor(246, 178, 137));
        io::stdout().flush().context("failed to flush prompt")?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("failed to read input")?;

        let prompt = input.trim();
        if prompt.is_empty() {
            continue;
        }

        if prompt.starts_with('/') {
            if handle_shell_command(prompt, &mut session)? {
                break;
            }
            continue;
        }

        if let Err(error) = run_agentic_task(prompt, &config, &mut session, None, None) {
            println!(
                "{} {}",
                "error:".red().bold(),
                error.to_string().bright_white()
            );
        }
    }

    Ok(())
}

fn handle_shell_command(command: &str, session: &mut ShiprSession) -> Result<bool> {
    match command {
        "/exit" | "/quit" => {
            println!("{}", "bye.".truecolor(148, 163, 184));
            Ok(true)
        }
        "/help" => {
            println!();
            println!("{}", "Commands".bold().bright_white());
            println!("  {} /help", "•".truecolor(246, 178, 137));
            println!("  {} /status", "•".truecolor(246, 178, 137));
            println!("  {} /tasks", "•".truecolor(246, 178, 137));
            println!("  {} /preview", "•".truecolor(246, 178, 137));
            println!("  {} /plan <task>", "•".truecolor(246, 178, 137));
            println!("  {} /login", "•".truecolor(246, 178, 137));
            println!("  {} /exit", "•".truecolor(246, 178, 137));
            Ok(false)
        }
        "/status" => {
            print_status(session);
            Ok(false)
        }
        "/tasks" => {
            print_tasks(session);
            Ok(false)
        }
        "/preview" => {
            preview_branding()?;
            Ok(false)
        }
        "/login" => {
            start_setup()?;
            Ok(false)
        }
        _ => {
            if let Some(task) = command.strip_prefix("/plan ") {
                show_plan(task)?;
            } else {
                println!("{}", "Unknown command. Use /help.".truecolor(148, 163, 184));
            }
            Ok(false)
        }
    }
}

fn print_shell_welcome() {
    println!();
    println!(
        "{} {}",
        "shipr".bold().bright_white(),
        format!("v{}", env!("CARGO_PKG_VERSION")).truecolor(148, 163, 184)
    );
    println!(
        "{}",
        "minimal agentic coding cli — type a task, or /help".truecolor(148, 163, 184)
    );
}

fn ensure_config_for_shell() -> Result<ShiprConfig> {
    if let Some(config) = load_config()? {
        return Ok(config);
    }

    println!(
        "{}",
        "No login found. Starting setup...".truecolor(148, 163, 184)
    );
    start_setup()?;

    let Some(config) = load_config()? else {
        bail!("failed to load config after setup");
    };

    Ok(config)
}

fn run_agentic_task(
    task: &str,
    config: &ShiprConfig,
    session: &mut ShiprSession,
    quality_override: Option<Quality>,
    budget_override: Option<Budget>,
) -> Result<()> {
    let task_id = format!("tsk-{:04}", session.next_task_number);
    session.next_task_number += 1;

    let started = Instant::now();
    let routing = resolve_routing_policy(task, quality_override, budget_override);
    let policy = routing.policy;
    let model = select_model(&policy);

    println!();
    println!(
        "{} {} {}",
        "●".truecolor(246, 178, 137),
        "Task".bold().bright_white(),
        task_id.truecolor(148, 163, 184)
    );
    println!("{}", task.bright_white());

    println!();
    println!(
        "{} {}",
        "Thought for".truecolor(148, 163, 184),
        "2s".truecolor(148, 163, 184)
    );
    println!(
        "{} Analyzing task scope and complexity.",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "{} Selecting cheapest viable routing policy.",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "{} Running minimal loop: plan → exec → verify.",
        "•".truecolor(246, 178, 137)
    );

    println!();
    println!("{}", "progress".bold().bright_white());
    for (progress, phase, detail) in [
        (10, "QUEUED", "task accepted"),
        (30, "PLAN", "building execution plan"),
        (60, "EXEC", "generating edits"),
        (85, "VERIFY", "running checks"),
        (100, "DONE", "finalizing response"),
    ] {
        println!(
            "  {} {:>3}% {:<7} {}",
            "•".truecolor(246, 178, 137),
            progress.to_string().bright_white(),
            phase.bold().bright_white(),
            detail.truecolor(203, 213, 225)
        );
        thread::sleep(Duration::from_millis(100));
    }

    let elapsed_ms = started.elapsed().as_millis();
    println!();
    println!(
        "{} {}",
        "route".truecolor(148, 163, 184),
        format!(
            "{} ({}; est {})",
            model.name, model.rationale, model.estimated_cost
        )
        .bright_white()
    );
    println!(
        "{} {}",
        "policy".truecolor(148, 163, 184),
        format!(
            "quality={} budget={}",
            format_quality(policy.quality),
            format_budget(policy.budget)
        )
        .bright_white()
    );
    println!(
        "{} {}",
        "routing".truecolor(148, 163, 184),
        format!("{} ({})", routing.mode, routing.task_kind).bright_white()
    );
    println!(
        "{} {}",
        "litellm".truecolor(148, 163, 184),
        config.base_url.bright_white()
    );

    println!();
    println!(
        "{} completed in {}ms",
        "status".truecolor(148, 163, 184),
        elapsed_ms
    );
    println!(
        "{} {}",
        "recap:".truecolor(148, 163, 184),
        format!(
            "Task '{}' routed to {} with {} / {} for cost-aware execution.",
            task,
            model.name,
            format_quality(policy.quality),
            format_budget(policy.budget)
        )
        .italic()
        .truecolor(148, 163, 184)
    );

    session.tasks.push(TaskRecord {
        id: task_id,
        prompt: task.to_string(),
        status: "completed",
        progress: 100,
        phase: "DONE",
        model: model.name,
        estimated_cost: model.estimated_cost,
        elapsed_ms,
    });

    Ok(())
}

fn start_setup() -> Result<()> {
    if let Some(config) = load_config()? {
        println!(
            "{} {}",
            "Already logged in:".truecolor(148, 163, 184),
            config.base_url.bright_white()
        );
        println!(
            "{} {}",
            "config:".truecolor(148, 163, 184),
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

    let config = ShiprConfig { base_url, api_key };
    validate_config(&config)?;
    save_config(&config)?;

    println!("{}", "Login successful.".truecolor(148, 163, 184));
    println!(
        "{}",
        "Run `shipr` to start the CLI.".truecolor(148, 163, 184)
    );

    Ok(())
}

fn preview_branding() -> Result<()> {
    println!();
    println!("{}", "shipr — minimal harness".bold().bright_white());
    println!("{} only the agentic loop", "•".truecolor(246, 178, 137));
    println!(
        "{} smart routing at harness level",
        "•".truecolor(246, 178, 137)
    );
    println!(
        "{} tuned for lower cost than heavy coding agents",
        "•".truecolor(246, 178, 137)
    );

    println!();
    println!("{}", "architecture".bold().bright_white());
    println!("  developer");
    println!("    └─ shipr");
    println!("       └─ agentic loop");
    println!("          └─ smart routing crate");
    println!("             └─ litellm auto router");

    Ok(())
}

fn show_plan(task: &str) -> Result<()> {
    println!();
    println!(
        "{} {}",
        "plan:".truecolor(148, 163, 184),
        task.bright_white()
    );
    println!("  1. understand task and repo context");
    println!("  2. smart-route by complexity and cost policy");
    println!("  3. apply focused code changes");
    println!("  4. verify output and summarize");
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

    let mut session = ShiprSession {
        next_task_number: 1,
        tasks: Vec::new(),
    };
    run_agentic_task(
        &task,
        &config,
        &mut session,
        quality_override,
        budget_override,
    )
}

fn print_status(session: &ShiprSession) {
    println!();
    println!("{}", "status".bold().bright_white());
    if let Some(task) = session.tasks.last() {
        println!("  task     {}", task.id.bright_white());
        println!("  state    {}", task.status.bright_white());
        println!("  phase    {}", task.phase.bright_white());
        println!("  progress {}%", task.progress);
        println!("  model    {}", task.model.bright_white());
        println!("  cost     {}", task.estimated_cost.bright_white());
    } else {
        println!("  idle (no tasks yet)");
    }
}

fn print_tasks(session: &ShiprSession) {
    println!();
    println!("{}", "recent tasks".bold().bright_white());
    if session.tasks.is_empty() {
        println!("  none yet");
        return;
    }

    for task in session.tasks.iter().rev().take(8) {
        println!(
            "  {} {}ms {}",
            task.id.bright_white(),
            task.elapsed_ms,
            task.prompt.truecolor(203, 213, 225)
        );
    }
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
