use std::{collections::HashMap, process::Stdio};

use anyhow::{anyhow, Result};
use krata::idm::{
    client::IdmClientStreamResponseHandle,
    internal::{
        exec_stream_request_update::Update, request::Request as RequestType,
        ExecStreamResponseUpdate,
    },
    internal::{response::Response as ResponseType, Request, Response},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    join,
    process::Command,
};

pub struct ZoneExecTask {
    pub handle: IdmClientStreamResponseHandle<Request>,
}

impl ZoneExecTask {
    pub async fn run(&self) -> Result<()> {
        let mut receiver = self.handle.take().await?;

        let Some(ref request) = self.handle.initial.request else {
            return Err(anyhow!("request was empty"));
        };

        let RequestType::ExecStream(update) = request else {
            return Err(anyhow!("request was not an exec update"));
        };

        let Some(Update::Start(ref start)) = update.update else {
            return Err(anyhow!("first request did not contain a start update"));
        };

        let mut cmd = start.command.clone();
        if cmd.is_empty() {
            return Err(anyhow!("command line was empty"));
        }
        let exe = cmd.remove(0);
        let mut env = HashMap::new();
        for entry in &start.environment {
            env.insert(entry.key.clone(), entry.value.clone());
        }

        if !env.contains_key("PATH") {
            env.insert(
                "PATH".to_string(),
                "/bin:/usr/bin:/usr/local/bin".to_string(),
            );
        }

        let dir = if start.working_directory.is_empty() {
            "/".to_string()
        } else {
            start.working_directory.clone()
        };

        let mut child = Command::new(exe)
            .args(cmd)
            .envs(env)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| anyhow!("failed to spawn: {}", error))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("stdin was missing"))?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("stdout was missing"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("stderr was missing"))?;

        let stdout_handle = self.handle.clone();
        let stdout_task = tokio::task::spawn(async move {
            let mut stdout_buffer = vec![0u8; 8 * 1024];
            loop {
                let Ok(size) = stdout.read(&mut stdout_buffer).await else {
                    break;
                };
                if size > 0 {
                    let response = Response {
                        response: Some(ResponseType::ExecStream(ExecStreamResponseUpdate {
                            exited: false,
                            exit_code: 0,
                            error: String::new(),
                            stdout: stdout_buffer[0..size].to_vec(),
                            stderr: vec![],
                        })),
                    };
                    let _ = stdout_handle.respond(response).await;
                } else {
                    break;
                }
            }
        });

        let stderr_handle = self.handle.clone();
        let stderr_task = tokio::task::spawn(async move {
            let mut stderr_buffer = vec![0u8; 8 * 1024];
            loop {
                let Ok(size) = stderr.read(&mut stderr_buffer).await else {
                    break;
                };
                if size > 0 {
                    let response = Response {
                        response: Some(ResponseType::ExecStream(ExecStreamResponseUpdate {
                            exited: false,
                            exit_code: 0,
                            error: String::new(),
                            stdout: vec![],
                            stderr: stderr_buffer[0..size].to_vec(),
                        })),
                    };
                    let _ = stderr_handle.respond(response).await;
                } else {
                    break;
                }
            }
        });

        let stdin_task = tokio::task::spawn(async move {
            loop {
                let Some(request) = receiver.recv().await else {
                    break;
                };

                let Some(RequestType::ExecStream(update)) = request.request else {
                    continue;
                };

                let Some(Update::Stdin(update)) = update.update else {
                    continue;
                };

                if stdin.write_all(&update.data).await.is_err() {
                    break;
                }
            }
        });

        let exit = child.wait().await?;
        let code = exit.code().unwrap_or(-1);

        let _ = join!(stdout_task, stderr_task);
        stdin_task.abort();

        let response = Response {
            response: Some(ResponseType::ExecStream(ExecStreamResponseUpdate {
                exited: true,
                exit_code: code,
                error: String::new(),
                stdout: vec![],
                stderr: vec![],
            })),
        };
        self.handle.respond(response).await?;

        Ok(())
    }
}
