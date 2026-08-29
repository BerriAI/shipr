use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const MAX_TOOL_OUTPUT_BYTES: usize = 32_000;
const SKIPPED_DIRECTORIES: [&str; 4] = [".git", "target", "node_modules", ".venv"];

#[derive(Debug)]
pub struct ToolRuntime {
    root: PathBuf,
}

#[derive(Debug)]
pub struct ToolOutput {
    pub content: String,
    pub summary: String,
    pub success: bool,
}

impl ToolRuntime {
    pub fn new(root: PathBuf) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("failed to open workspace {}", root.display()))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn requires_approval(name: &str) -> bool {
        matches!(name, "write_file" | "replace_in_file" | "run_command")
    }

    pub fn approval_description(name: &str, arguments: &str) -> String {
        let parsed: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);
        match name {
            "write_file" => format!(
                "write {}",
                parsed["path"].as_str().unwrap_or("the requested file")
            ),
            "replace_in_file" => format!(
                "edit {}",
                parsed["path"].as_str().unwrap_or("the requested file")
            ),
            "run_command" => {
                let program = parsed["program"].as_str().unwrap_or("command");
                let args = parsed["args"]
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                format!("run {program} {args}").trim().to_string()
            }
            _ => format!("use {name}"),
        }
    }

    pub fn execute(&self, name: &str, arguments: &str) -> Result<ToolOutput> {
        match name {
            "list_files" => self.list_files(parse_arguments(arguments)?),
            "read_file" => self.read_file(parse_arguments(arguments)?),
            "search" => self.search(parse_arguments(arguments)?),
            "write_file" => self.write_file(parse_arguments(arguments)?),
            "replace_in_file" => self.replace_in_file(parse_arguments(arguments)?),
            "run_command" => self.run_command(parse_arguments(arguments)?),
            _ => bail!("unknown tool: {name}"),
        }
    }

    fn list_files(&self, arguments: ListFilesArguments) -> Result<ToolOutput> {
        let relative_path = arguments.path.unwrap_or_else(|| ".".to_string());
        let path = self.resolve_existing_path(&relative_path)?;
        if !path.is_dir() {
            bail!("{} is not a directory", relative_path);
        }

        let mut files = Vec::new();
        collect_files(&self.root, &path, &mut files)?;
        files.sort();
        let total = files.len();
        files.truncate(arguments.limit.unwrap_or(300).min(1_000));
        let content = truncate_output(files.join("\n"));
        Ok(ToolOutput {
            content,
            summary: format!("listed {total} files under {relative_path}"),
            success: true,
        })
    }

    fn read_file(&self, arguments: ReadFileArguments) -> Result<ToolOutput> {
        let path = self.resolve_existing_path(&arguments.path)?;
        if !path.is_file() {
            bail!("{} is not a file", arguments.path);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", arguments.path))?;
        let lines: Vec<&str> = content.lines().collect();
        let start = arguments.start_line.unwrap_or(1).max(1);
        let end = arguments.end_line.unwrap_or(lines.len()).min(lines.len());
        if start > end.saturating_add(1) {
            bail!("start_line must be before end_line");
        }

        let selected = lines
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                let line_number = index + 1;
                line_number >= start && line_number <= end
            })
            .map(|(index, line)| format!("{:>5} | {line}", index + 1))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutput {
            content: truncate_output(selected),
            summary: format!("read {} lines {start}-{end}", arguments.path),
            success: true,
        })
    }

    fn search(&self, arguments: SearchArguments) -> Result<ToolOutput> {
        let relative_path = arguments.path.unwrap_or_else(|| ".".to_string());
        let path = self.resolve_existing_path(&relative_path)?;
        let output = Command::new("rg")
            .args(["-n", "--hidden", "--glob", "!.git", "--glob", "!target"])
            .arg("--")
            .arg(&arguments.query)
            .arg(&path)
            .current_dir(&self.root)
            .output()
            .context("failed to run rg; install ripgrep to use search")?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() && output.status.code() != Some(1) {
            bail!("search failed: {}", stderr.trim());
        }

        let matches = stdout.lines().count();
        Ok(ToolOutput {
            content: if stdout.is_empty() {
                "No matches found.".to_string()
            } else {
                truncate_output(stdout)
            },
            summary: format!("found {matches} matches for {:?}", arguments.query),
            success: true,
        })
    }

    fn write_file(&self, arguments: WriteFileArguments) -> Result<ToolOutput> {
        let path = self.resolve_writable_path(&arguments.path)?;
        let parent = path.parent().context("file path has no parent")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
        self.ensure_inside_workspace(parent)?;
        fs::write(&path, arguments.content.as_bytes())
            .with_context(|| format!("failed to write {}", arguments.path))?;
        Ok(ToolOutput {
            content: format!(
                "Wrote {} bytes to {}.",
                arguments.content.len(),
                arguments.path
            ),
            summary: format!("wrote {}", arguments.path),
            success: true,
        })
    }

    fn replace_in_file(&self, arguments: ReplaceInFileArguments) -> Result<ToolOutput> {
        if arguments.old_text.is_empty() {
            bail!("old_text cannot be empty");
        }
        let path = self.resolve_existing_path(&arguments.path)?;
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", arguments.path))?;
        let occurrences = content.matches(&arguments.old_text).count();
        if occurrences != 1 {
            bail!("old_text must match exactly once; found {occurrences} matches");
        }
        let updated = content.replacen(&arguments.old_text, &arguments.new_text, 1);
        fs::write(&path, updated)
            .with_context(|| format!("failed to update {}", arguments.path))?;
        Ok(ToolOutput {
            content: format!("Updated {}.", arguments.path),
            summary: format!("edited {}", arguments.path),
            success: true,
        })
    }

    fn run_command(&self, arguments: RunCommandArguments) -> Result<ToolOutput> {
        if arguments.program.trim().is_empty() {
            bail!("program cannot be empty");
        }
        let output = Command::new(&arguments.program)
            .args(&arguments.args)
            .current_dir(&self.root)
            .output()
            .with_context(|| format!("failed to run {}", arguments.program))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!(
            "$ {} {}\n{}{}",
            arguments.program,
            arguments.args.join(" "),
            stdout,
            stderr
        );
        let exit_code = output.status.code().unwrap_or(-1);
        Ok(ToolOutput {
            content: truncate_output(combined),
            summary: format!("{} exited with {exit_code}", arguments.program),
            success: output.status.success(),
        })
    }

    fn resolve_existing_path(&self, relative_path: &str) -> Result<PathBuf> {
        let candidate = self.resolve_lexical_path(relative_path)?;
        let canonical = candidate
            .canonicalize()
            .with_context(|| format!("path does not exist: {relative_path}"))?;
        self.ensure_inside_workspace(&canonical)?;
        Ok(canonical)
    }

    fn resolve_writable_path(&self, relative_path: &str) -> Result<PathBuf> {
        let candidate = self.resolve_lexical_path(relative_path)?;
        if candidate.exists() {
            self.ensure_inside_workspace(&candidate.canonicalize()?)?;
        } else if let Some(existing_ancestor) = candidate.ancestors().find(|path| path.exists()) {
            self.ensure_inside_workspace(&existing_ancestor.canonicalize()?)?;
        }
        Ok(candidate)
    }

    fn resolve_lexical_path(&self, relative_path: &str) -> Result<PathBuf> {
        let path = Path::new(relative_path);
        if relative_path.trim().is_empty() || path.is_absolute() {
            bail!("tool paths must be relative to the workspace");
        }
        for component in path.components() {
            match component {
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    bail!("path cannot leave the workspace")
                }
                Component::Normal(name) if name == ".git" => {
                    bail!("editing .git is not allowed")
                }
                _ => {}
            }
        }
        Ok(self.root.join(path))
    }

    fn ensure_inside_workspace(&self, path: &Path) -> Result<()> {
        if !path.starts_with(&self.root) {
            bail!("path cannot leave workspace {}", self.root.display());
        }
        Ok(())
    }
}

pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool_definition(
            "list_files",
            "List files in the workspace. Build artifacts and dependency directories are skipped.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative directory, defaults to ." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000 }
                }
            }),
        ),
        tool_definition(
            "read_file",
            "Read a UTF-8 text file with line numbers.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "start_line": { "type": "integer", "minimum": 1 },
                    "end_line": { "type": "integer", "minimum": 1 }
                },
                "required": ["path"]
            }),
        ),
        tool_definition(
            "search",
            "Search workspace text with ripgrep.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "path": { "type": "string", "description": "Relative file or directory, defaults to ." }
                },
                "required": ["query"]
            }),
        ),
        tool_definition(
            "write_file",
            "Create or overwrite a UTF-8 file. Requires user approval.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
        ),
        tool_definition(
            "replace_in_file",
            "Replace one exact text occurrence in a UTF-8 file. Requires user approval.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_text": { "type": "string" },
                    "new_text": { "type": "string" }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        ),
        tool_definition(
            "run_command",
            "Run one program directly in the workspace without a shell. Requires user approval.",
            json!({
                "type": "object",
                "properties": {
                    "program": { "type": "string", "description": "Executable name, such as cargo or git" },
                    "args": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["program", "args"]
            }),
        ),
    ]
}

fn tool_definition(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    })
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to list {}", directory.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let name = entry.file_name();
            if SKIPPED_DIRECTORIES.iter().any(|skipped| name == *skipped) {
                continue;
            }
            collect_files(root, &path, files)?;
        } else if file_type.is_file() {
            files.push(path.strip_prefix(root)?.display().to_string());
        }
    }
    Ok(())
}

fn parse_arguments<T: for<'de> Deserialize<'de>>(arguments: &str) -> Result<T> {
    serde_json::from_str(arguments).context("tool arguments were not valid JSON")
}

fn truncate_output(mut output: String) -> String {
    if output.len() <= MAX_TOOL_OUTPUT_BYTES {
        return output;
    }
    let mut boundary = MAX_TOOL_OUTPUT_BYTES;
    while !output.is_char_boundary(boundary) {
        boundary -= 1;
    }
    output.truncate(boundary);
    output.push_str("\n… output truncated by shipr");
    output
}

#[derive(Debug, Deserialize)]
struct ListFilesArguments {
    path: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ReadFileArguments {
    path: String,
    start_line: Option<usize>,
    end_line: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SearchArguments {
    query: String,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WriteFileArguments {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ReplaceInFileArguments {
    path: String,
    old_text: String,
    new_text: String,
}

#[derive(Debug, Deserialize)]
struct RunCommandArguments {
    program: String,
    #[serde(default)]
    args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_reads_and_replaces_workspace_files() {
        let directory = tempdir().expect("temp directory");
        let runtime = ToolRuntime::new(directory.path().to_path_buf()).expect("runtime");

        runtime
            .execute(
                "write_file",
                r#"{"path":"src/main.rs","content":"fn main() {}\n"}"#,
            )
            .expect("write file");
        let read = runtime
            .execute("read_file", r#"{"path":"src/main.rs"}"#)
            .expect("read file");
        assert!(read.content.contains("fn main() {}"));

        runtime
            .execute(
                "replace_in_file",
                r#"{"path":"src/main.rs","old_text":"main","new_text":"start"}"#,
            )
            .expect("replace text");
        assert_eq!(
            fs::read_to_string(directory.path().join("src/main.rs")).expect("updated file"),
            "fn start() {}\n"
        );
    }

    #[test]
    fn rejects_paths_outside_workspace() {
        let directory = tempdir().expect("temp directory");
        let runtime = ToolRuntime::new(directory.path().to_path_buf()).expect("runtime");

        let error = runtime
            .execute("read_file", r#"{"path":"../secret"}"#)
            .expect_err("path should be rejected");

        assert!(error.to_string().contains("workspace"));
    }

    #[test]
    fn exposes_six_basic_coding_tools() {
        assert_eq!(tool_definitions().len(), 6);
    }
}
