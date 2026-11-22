use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use std::{ffi::OsString, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Harness {
    Opencode,
    #[value(name = "claude-code")]
    ClaudeCode,
    Codex,
    #[value(name = "chatgpt-desktop")]
    ChatGptDesktop,
    All,
}

pub fn configure(harnesses: &[Harness], port: u16, dry_run: bool) -> Result<()> {
    anyhow::ensure!(
        port != 0,
        "setup requires the daemon's actual listening port"
    );
    let url = format!("http://127.0.0.1:{port}/mcp");
    let expanded = expand(harnesses);
    for harness in expanded {
        let (program, arguments): (&str, Vec<OsString>) = match harness {
            Harness::Opencode => (
                "opencode",
                vec![
                    "mcp".into(),
                    "add".into(),
                    "span".into(),
                    "--global".into(),
                    "--url".into(),
                    url.as_str().into(),
                    "--header".into(),
                    "Authorization=Bearer {env:SPAN_HTTP_TOKEN}".into(),
                ],
            ),
            Harness::ClaudeCode => (
                "claude",
                vec![
                    "mcp".into(),
                    "add-json".into(),
                    "--scope".into(),
                    "user".into(),
                    "span".into(),
                    serde_json::json!({"type":"http", "url":url, "headers":{"Authorization":"Bearer ${SPAN_HTTP_TOKEN}"}}).to_string().into(),
                ],
            ),
            Harness::Codex | Harness::ChatGptDesktop => (
                "codex",
                vec![
                    "mcp".into(),
                    "add".into(),
                    "span".into(),
                    "--url".into(),
                    url.as_str().into(),
                    "--bearer-token-env-var".into(),
                    "SPAN_HTTP_TOKEN".into(),
                ],
            ),
            Harness::All => unreachable!(),
        };
        println!("{}", display_command(program, &arguments));
        if dry_run {
            continue;
        }
        let status = match Command::new(program).args(&arguments).status() {
            Ok(status) => status,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && harnesses.contains(&Harness::All) =>
            {
                eprintln!("skipping {program}: command not found");
                continue;
            }
            Err(error) => return Err(error).with_context(|| format!("failed to start {program}")),
        };
        if !status.success() {
            bail!("{program} exited with status {status}");
        }
        if harness == Harness::ChatGptDesktop {
            println!(
                "ChatGPT Desktop shares this Codex-host MCP configuration; restart the desktop app."
            );
        }
    }
    Ok(())
}

fn expand(harnesses: &[Harness]) -> Vec<Harness> {
    if harnesses.contains(&Harness::All) {
        vec![Harness::Opencode, Harness::ClaudeCode, Harness::Codex]
    } else {
        let mut result = Vec::new();
        for harness in harnesses {
            if !result.contains(harness) {
                result.push(*harness);
            }
        }
        result
    }
}

fn display_command(program: &str, arguments: &[OsString]) -> String {
    std::iter::once(program.to_owned())
        .chain(
            arguments
                .iter()
                .map(|argument| shell_quote(&argument.to_string_lossy())),
        )
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "_./:-".contains(character))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
