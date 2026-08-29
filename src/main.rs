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
    print_header("shipr", "agentic cli");

    let config = ensure_config_for_shell()?;
    let mut session = ShiprSession {
        next_task_number: 1,
        tasks: Vec::new(),
    };

    print_kv("mode", "task-driven");
    print_kv(
        "hint",
        "type a task directly. commands: /help /status /tasks /exit",
    );

    loop {
        print!("{}", "shipr> ".truecolor(246, 178, 137));
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
            match prompt {
                "/exit" | "/quit" => {
                    print_kv("bye", "see you next ship");
                    break;
                }
                "/help" => {
                    print_section("commands");
                    println!(
                        "  {} type any text to run an agentic task",
                        "•".truecolor(246, 178, 137)
                    );
                    println!("  {} /status", "•".truecolor(246, 178, 137));
                    println!("  {} /tasks", "•".truecolor(246, 178, 137));
                    println!("  {} /preview", "•".truecolor(246, 178, 137));
                    println!("  {} /login", "•".truecolor(246, 178, 137));
                    println!("  {} /plan <task>", "•".truecolor(246, 178, 137));
                    println!("  {} /exit", "•".truecolor(246, 178, 137));
                }
                "/status" => print_status(&session),
                "/tasks" => print_tasks(&session),
                "/preview" => {
                    if let Err(error) = preview_branding() {
                        print_kv("error", &error.to_string());
                    }
                }
                "/login" => {
                    if let Err(error) = start_setup() {
                        print_kv("error", &error.to_string());
                    }
                }
                _ => {
                    if let Some(task) = prompt.strip_prefix("/plan ") {
                        if let Err(error) = show_plan(task) {
                            print_kv("error", &error.to_string());
                        }
                    } else {
                        print_kv("error", "unknown command. use /help");
                    }
                }
            }
            continue;
        }

        if let Err(error) = run_agentic_task(prompt, &config, &mut session) {
            print_kv("error", &error.to_string());
        }
    }

    Ok(())
}

fn ensure_config_for_shell() -> Result<ShiprConfig> {
    if let Some(config) = load_config()? {
        return Ok(config);
    }

    print_kv("login", "first run detected");
    start_setup()?;

    let Some(config) = load_config()? else {
        bail!("failed to load config after setup");
    };

    Ok(config)
}

fn run_agentic_task(task: &str, config: &ShiprConfig, session: &mut ShiprSession) -> Result<()> {
    let task_id = format!("tsk-{:04}", session.next_task_number);
    session.next_task_number += 1;

    let started = Instant::now();
    let routing = resolve_routing_policy(task, None, None);
    let policy = routing.policy;
    let model = select_model(&policy);

    print_header("shipr", "task");
    print_kv("id", &task_id);
    print_kv("task", task);
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
        "route",
        &format!(
            "{} ({}, est {})",
            model.name, model.rationale, model.estimated_cost
        ),
    );
    print_kv(
        "routing",
        &format!("{} ({})", routing.mode, routing.task_kind),
    );

    print_section("progress");
    for (progress, phase, detail) in [
        (10, "QUEUED", "task accepted"),
        (30, "PLAN", "building execution plan"),
        (60, "EXEC", "generating edits"),
        (85, "VERIFY", "running checks"),
        (100, "DONE", "finalizing response"),
    ] {
        print_progress(progress, phase, detail);
        thread::sleep(Duration::from_millis(120));
    }

    let elapsed_ms = started.elapsed().as_millis();
    print_kv("status", &format!("completed in {elapsed_ms}ms"));

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

fn print_status(session: &ShiprSession) {
    print_section("status");
    if let Some(task) = session.tasks.last() {
        print_kv("task", &task.id);
        print_kv("state", task.status);
        print_kv("phase", task.phase);
        print_kv("progress", &format!("{}%", task.progress));
        print_kv("model", task.model);
        print_kv("cost", task.estimated_cost);
    } else {
        print_kv("state", "idle (no tasks yet)");
    }
}

fn print_tasks(session: &ShiprSession) {
    print_section("recent tasks");
    if session.tasks.is_empty() {
        print_kv("tasks", "none yet");
        return;
    }

    for task in session.tasks.iter().rev().take(5) {
        println!(
            "  {} {} {} {}",
            "•".truecolor(246, 178, 137),
            task.id.truecolor(226, 232, 240),
            format!("{}ms", task.elapsed_ms).truecolor(148, 163, 184),
            task.prompt.truecolor(203, 213, 225)
        );
    }
}

fn print_progress(progress: u8, phase: &str, detail: &str) {
    println!(
        "  {} {:>3}% {:<7} {}",
        "•".truecolor(246, 178, 137),
        progress.to_string().truecolor(226, 232, 240),
        phase.bold().bright_white(),
        detail.truecolor(203, 213, 225)
    );
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
    print_kv("next", "shipr");

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
        "shipr -> type your task at prompt".truecolor(148, 163, 184)
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

    let mut session = ShiprSession {
        next_task_number: 1,
        tasks: Vec::new(),
    };
    let task_label = if quality_override.is_none() && budget_override.is_none() {
        task
    } else {
        format!(
            "{} (overrides: quality={:?} budget={:?})",
            task, quality_override, budget_override
        )
    };
    run_agentic_task(&task_label, &config, &mut session)
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
