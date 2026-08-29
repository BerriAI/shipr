use crate::tools::{ToolRuntime, tool_definitions};
use anyhow::{Context, Result, bail};
use reqwest::blocking::{Client, Response};
use serde::Serialize;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

const MAX_AGENT_TURNS: usize = 20;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub route: String,
    pub workspace: PathBuf,
}

#[derive(Debug)]
pub enum AgentEvent {
    Activity(String),
    ApprovalRequested {
        description: String,
        response: Sender<bool>,
    },
    ToolFinished {
        name: String,
        summary: String,
        success: bool,
    },
    ResponseStarted,
    ResponseDelta(String),
}

#[derive(Debug)]
pub struct AgentSummary {
    pub model: String,
    pub route: String,
}

pub fn run_agent(
    config: AgentConfig,
    prompt: String,
    mut emit: impl FnMut(AgentEvent) -> bool,
) -> Result<AgentSummary> {
    let runtime = ToolRuntime::new(config.workspace.clone())?;
    let client = Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .context("failed to create LiteLLM client")?;
    let endpoint = chat_completions_endpoint(&config.base_url);
    let mut messages = vec![
        json!({
            "role": "system",
            "content": system_prompt(runtime.root().display().to_string())
        }),
        json!({ "role": "user", "content": prompt }),
    ];

    for _ in 0..MAX_AGENT_TURNS {
        if !emit(AgentEvent::Activity(format!(
            "Asking {} via {}",
            config.model, config.route
        ))) {
            bail!("task cancelled");
        }
        let turn = request_turn(&client, &endpoint, &config, &messages, &mut emit)?;
        messages.push(turn.as_message());

        if turn.tool_calls.is_empty() {
            if turn.content.trim().is_empty() {
                bail!("LiteLLM returned an empty response");
            }
            return Ok(AgentSummary {
                model: config.model,
                route: config.route,
            });
        }

        for tool_call in turn.tool_calls {
            let description = ToolRuntime::approval_description(
                &tool_call.function.name,
                &tool_call.function.arguments,
            );
            if !emit(AgentEvent::Activity(description.clone())) {
                bail!("task cancelled");
            }

            let approved = if ToolRuntime::requires_approval(&tool_call.function.name) {
                request_approval(&description, &mut emit)?
            } else {
                true
            };
            let result = if approved {
                runtime.execute(&tool_call.function.name, &tool_call.function.arguments)
            } else {
                Err(anyhow::anyhow!("user denied this tool call"))
            };

            let (content, summary, success) = match result {
                Ok(output) => (output.content, output.summary, output.success),
                Err(error) => {
                    let message = format!("Tool error: {error:#}");
                    (message.clone(), message, false)
                }
            };
            if !emit(AgentEvent::ToolFinished {
                name: tool_call.function.name.clone(),
                summary,
                success,
            }) {
                bail!("task cancelled");
            }
            messages.push(json!({
                "role": "tool",
                "tool_call_id": tool_call.id,
                "name": tool_call.function.name,
                "content": content
            }));
        }
    }

    bail!("agent stopped after {MAX_AGENT_TURNS} turns")
}

fn request_turn(
    client: &Client,
    endpoint: &str,
    config: &AgentConfig,
    messages: &[Value],
    emit: &mut impl FnMut(AgentEvent) -> bool,
) -> Result<AssistantTurn> {
    let response = client
        .post(endpoint)
        .bearer_auth(&config.api_key)
        .json(&json!({
            "model": config.model,
            "messages": messages,
            "tools": tool_definitions(),
            "tool_choice": "auto",
            "stream": true,
            "metadata": {
                "shiprr_route": config.route,
                "shiprr_client": "shipr-cli"
            }
        }))
        .send()
        .context("failed to reach LiteLLM")?;
    parse_streaming_response(response, emit)
}

fn parse_streaming_response(
    response: Response,
    emit: &mut impl FnMut(AgentEvent) -> bool,
) -> Result<AssistantTurn> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        bail!("LiteLLM returned {status}: {body}");
    }

    let reader = BufReader::new(response);
    let mut content = String::new();
    let mut response_started = false;
    let mut tool_calls: Vec<ToolCallBuilder> = Vec::new();

    for line in reader.lines() {
        let line = line.context("failed while reading LiteLLM stream")?;
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            break;
        }
        let event: Value = serde_json::from_str(data).context("invalid LiteLLM stream event")?;
        if let Some(error) = event.get("error") {
            bail!("LiteLLM stream error: {error}");
        }
        let Some(delta) = event
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"))
        else {
            continue;
        };

        if let Some(fragment) = delta.get("content").and_then(Value::as_str) {
            if !response_started {
                if !emit(AgentEvent::ResponseStarted) {
                    bail!("task cancelled");
                }
                response_started = true;
            }
            content.push_str(fragment);
            if !emit(AgentEvent::ResponseDelta(fragment.to_string())) {
                bail!("task cancelled");
            }
        }

        if let Some(fragments) = delta.get("tool_calls").and_then(Value::as_array) {
            for fragment in fragments {
                accumulate_tool_call(&mut tool_calls, fragment)?;
            }
        }
    }

    let tool_calls = tool_calls
        .into_iter()
        .map(ToolCallBuilder::finish)
        .collect::<Result<Vec<_>>>()?;
    Ok(AssistantTurn {
        content,
        tool_calls,
    })
}

fn accumulate_tool_call(tool_calls: &mut Vec<ToolCallBuilder>, fragment: &Value) -> Result<()> {
    let index = fragment
        .get("index")
        .and_then(Value::as_u64)
        .context("tool call stream fragment omitted index")? as usize;
    while tool_calls.len() <= index {
        tool_calls.push(ToolCallBuilder::default());
    }
    let builder = &mut tool_calls[index];
    if let Some(id) = fragment.get("id").and_then(Value::as_str) {
        builder.id.push_str(id);
    }
    if let Some(function) = fragment.get("function") {
        if let Some(name) = function.get("name").and_then(Value::as_str) {
            builder.name.push_str(name);
        }
        if let Some(arguments) = function.get("arguments").and_then(Value::as_str) {
            builder.arguments.push_str(arguments);
        }
    }
    Ok(())
}

fn request_approval(description: &str, emit: &mut impl FnMut(AgentEvent) -> bool) -> Result<bool> {
    let (sender, receiver) = std::sync::mpsc::channel();
    if !emit(AgentEvent::ApprovalRequested {
        description: description.to_string(),
        response: sender,
    }) {
        return Ok(false);
    }
    receiver.recv().context("approval prompt closed")
}

fn system_prompt(workspace: String) -> String {
    format!(
        "You are Shiprr, a minimal coding agent operating in {workspace}. \
Use tools to inspect the repository before changing it. Make focused edits that solve the user's task. \
All file paths must be relative to the workspace. Prefer replace_in_file for small edits and write_file for new files. \
Run relevant checks after editing. Never claim a file or command changed unless the matching tool succeeded. \
When the task is complete, respond with a concise summary and validation results."
    )
}

fn chat_completions_endpoint(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

#[derive(Debug)]
struct AssistantTurn {
    content: String,
    tool_calls: Vec<ToolCall>,
}

impl AssistantTurn {
    fn as_message(&self) -> Value {
        json!({
            "role": "assistant",
            "content": if self.content.is_empty() { Value::Null } else { Value::String(self.content.clone()) },
            "tool_calls": self.tool_calls
        })
    }
}

#[derive(Debug, Clone, Serialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Default)]
struct ToolCallBuilder {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCallBuilder {
    fn finish(self) -> Result<ToolCall> {
        if self.id.is_empty() || self.name.is_empty() {
            bail!("LiteLLM returned an incomplete tool call");
        }
        Ok(ToolCall {
            id: self.id,
            kind: "function",
            function: FunctionCall {
                name: self.name,
                arguments: self.arguments,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn appends_v1_for_plain_gateway_urls() {
        assert_eq!(
            chat_completions_endpoint("http://localhost:4000/"),
            "http://localhost:4000/v1/chat/completions"
        );
    }

    #[test]
    fn preserves_existing_v1_prefix() {
        assert_eq!(
            chat_completions_endpoint("http://localhost:4000/v1"),
            "http://localhost:4000/v1/chat/completions"
        );
    }

    #[test]
    fn combines_streamed_tool_call_fragments() {
        let mut calls = Vec::new();
        accumulate_tool_call(
            &mut calls,
            &json!({ "index": 0, "id": "call_1", "function": { "name": "write_", "arguments": "{\"pa" } }),
        )
        .expect("first fragment");
        accumulate_tool_call(
            &mut calls,
            &json!({ "index": 0, "function": { "name": "file", "arguments": "th\":\"x\"}" } }),
        )
        .expect("second fragment");

        let call = calls.pop().expect("tool call").finish().expect("complete");
        assert_eq!(call.function.name, "write_file");
        assert_eq!(call.function.arguments, r#"{"path":"x"}"#);
    }

    #[test]
    fn runs_an_approved_file_tool_then_streams_the_answer() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock gateway");
        let address = listener.local_addr().expect("gateway address");
        let server = thread::spawn(move || {
            serve_sse(
                &listener,
                &[
                    json!({
                        "choices": [{
                            "delta": {
                                "tool_calls": [{
                                    "index": 0,
                                    "id": "call_1",
                                    "function": {
                                        "name": "write_file",
                                        "arguments": "{\"path\":\"created.txt\","
                                    }
                                }]
                            }
                        }]
                    }),
                    json!({
                        "choices": [{
                            "delta": {
                                "tool_calls": [{
                                    "index": 0,
                                    "function": { "arguments": "\"content\":\"hello\"}" }
                                }]
                            }
                        }]
                    }),
                ],
            );
            serve_sse(
                &listener,
                &[
                    json!({ "choices": [{ "delta": { "content": "Created " } }] }),
                    json!({ "choices": [{ "delta": { "content": "the file." } }] }),
                ],
            );
        });
        let workspace = tempdir().expect("workspace");
        let mut streamed = String::new();
        let mut tool_succeeded = false;

        run_agent(
            AgentConfig {
                base_url: format!("http://{address}"),
                api_key: "test-key".to_string(),
                model: "test-model".to_string(),
                route: "test-route".to_string(),
                workspace: workspace.path().to_path_buf(),
            },
            "create a file".to_string(),
            |event| {
                match event {
                    AgentEvent::ApprovalRequested { response, .. } => {
                        response.send(true).expect("approve tool");
                    }
                    AgentEvent::ToolFinished { success, .. } => tool_succeeded = success,
                    AgentEvent::ResponseDelta(delta) => streamed.push_str(&delta),
                    _ => {}
                }
                true
            },
        )
        .expect("agent run");
        server.join().expect("mock gateway thread");

        assert!(tool_succeeded);
        assert_eq!(streamed, "Created the file.");
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("created.txt")).expect("created file"),
            "hello"
        );
    }

    fn serve_sse(listener: &TcpListener, events: &[Value]) {
        let (mut stream, _) = listener.accept().expect("gateway request");
        let mut request = [0_u8; 16_384];
        let _ = stream.read(&mut request).expect("read request");
        let body = events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect::<String>()
            + "data: [DONE]\n\n";
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write response");
    }
}
