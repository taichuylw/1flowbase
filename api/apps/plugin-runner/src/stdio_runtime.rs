use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use plugin_framework::{
    error::{FrameworkResult, PluginFrameworkError},
    provider_contract::{
        ProviderInvocationResult, ProviderRuntimeError, ProviderRuntimeErrorKind,
        ProviderRuntimeLine, ProviderStdioError, ProviderStdioRequest, ProviderStdioResponse,
        ProviderStreamEvent,
    },
    PluginRuntimeLimits,
};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
};

#[derive(Debug, Clone, PartialEq)]
pub struct StreamingProviderOutput {
    pub events: Vec<ProviderStreamEvent>,
    pub result: ProviderInvocationResult,
}

#[derive(Debug)]
pub struct ProviderWorker {
    executable_path: PathBuf,
    limits: PluginRuntimeLimits,
    process: Option<ProviderWorkerProcess>,
}

#[derive(Debug)]
struct ProviderWorkerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}

impl ProviderWorker {
    pub fn new(executable_path: PathBuf, limits: PluginRuntimeLimits) -> Self {
        Self {
            executable_path,
            limits,
            process: None,
        }
    }

    pub async fn call(&mut self, request: &ProviderStdioRequest) -> FrameworkResult<Value> {
        let timeout_ms = self.limits.timeout_ms.unwrap_or(30_000);
        match tokio::time::timeout(Duration::from_millis(timeout_ms), self.call_inner(request))
            .await
        {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => {
                self.stop().await;
                Err(error)
            }
            Err(_) => {
                self.stop().await;
                Err(provider_timeout_error())
            }
        }
    }

    pub async fn call_streaming(
        &mut self,
        request: &ProviderStdioRequest,
        live_events: Option<tokio::sync::mpsc::UnboundedSender<ProviderStreamEvent>>,
    ) -> FrameworkResult<StreamingProviderOutput> {
        let timeout_ms = self.limits.timeout_ms.unwrap_or(30_000);
        match tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            self.call_streaming_inner(request, live_events),
        )
        .await
        {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(error)) => {
                self.stop().await;
                Err(error)
            }
            Err(_) => {
                self.stop().await;
                Err(provider_timeout_error())
            }
        }
    }

    pub async fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.child.kill().await;
        }
    }

    async fn call_inner(&mut self, request: &ProviderStdioRequest) -> FrameworkResult<Value> {
        let executable_path = self.executable_path.clone();
        let process = self.ensure_process()?;
        write_worker_request(&executable_path, &mut process.stdin, request).await?;

        while let Some(line) =
            process.stdout.next_line().await.map_err(|error| {
                PluginFrameworkError::io(Some(&executable_path), error.to_string())
            })?
        {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            return parse_stdio_response_line(&executable_path, trimmed);
        }

        Err(worker_ended_without_output_error())
    }

    async fn call_streaming_inner(
        &mut self,
        request: &ProviderStdioRequest,
        live_events: Option<tokio::sync::mpsc::UnboundedSender<ProviderStreamEvent>>,
    ) -> FrameworkResult<StreamingProviderOutput> {
        let executable_path = self.executable_path.clone();
        let process = self.ensure_process()?;
        write_worker_request(&executable_path, &mut process.stdin, request).await?;

        let mut events = Vec::new();
        let mut result = None;

        while let Some(line) =
            process.stdout.next_line().await.map_err(|error| {
                PluginFrameworkError::io(Some(&executable_path), error.to_string())
            })?
        {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let runtime_line =
                serde_json::from_str::<ProviderRuntimeLine>(trimmed).map_err(|error| {
                    PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
                        "invalid_provider_ndjson",
                        format!("invalid provider ndjson: {error}"),
                        Some(trimmed),
                    ))
                })?;
            match runtime_line {
                ProviderRuntimeLine::Result { result: value } => {
                    result = Some(value);
                    break;
                }
                other => {
                    if let Some(event) = other.into_stream_event() {
                        if let Some(live_events) = &live_events {
                            let _ = live_events.send(event.clone());
                        }
                        events.push(event);
                    }
                }
            }
        }

        let result = result.ok_or_else(worker_ended_without_result_error)?;
        Ok(StreamingProviderOutput { events, result })
    }

    fn ensure_process(&mut self) -> FrameworkResult<&mut ProviderWorkerProcess> {
        let should_start = match self.process.as_mut() {
            Some(process) => process
                .child
                .try_wait()
                .map_err(|error| {
                    PluginFrameworkError::io(Some(&self.executable_path), error.to_string())
                })?
                .is_some(),
            None => true,
        };
        if should_start {
            self.process = Some(spawn_worker_process(&self.executable_path, &self.limits)?);
        }
        Ok(self
            .process
            .as_mut()
            .expect("worker process is initialized"))
    }
}

pub async fn call_executable(
    executable_path: &Path,
    request: &ProviderStdioRequest,
    limits: &PluginRuntimeLimits,
) -> FrameworkResult<Value> {
    let mut command = Command::new(executable_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    apply_memory_limit(&mut command, limits.memory_bytes)?;

    let mut child = command
        .spawn()
        .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        let mut payload = serde_json::to_vec(request)
            .map_err(|error| PluginFrameworkError::serialization(None, error.to_string()))?;
        payload.push(b'\n');
        stdin
            .write_all(&payload)
            .await
            .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;
    }

    let output = tokio::time::timeout(
        Duration::from_millis(limits.timeout_ms.unwrap_or(30_000)),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| provider_timeout_error())?
    .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;

    parse_stdio_response(executable_path, &output.stdout, &output.stderr)
}

pub async fn call_executable_streaming(
    executable_path: &Path,
    request: &ProviderStdioRequest,
    limits: &PluginRuntimeLimits,
    live_events: Option<tokio::sync::mpsc::UnboundedSender<ProviderStreamEvent>>,
) -> FrameworkResult<StreamingProviderOutput> {
    let mut command = Command::new(executable_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    apply_memory_limit(&mut command, limits.memory_bytes)?;

    let mut child = command
        .spawn()
        .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        let mut payload = serde_json::to_vec(request)
            .map_err(|error| PluginFrameworkError::serialization(None, error.to_string()))?;
        payload.push(b'\n');
        stdin
            .write_all(&payload)
            .await
            .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;
    }

    let stdout = child.stdout.take().ok_or_else(|| {
        PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
            "provider_runtime",
            "provider runtime stdout was not captured",
            None,
        ))
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
            "provider_runtime",
            "provider runtime stderr was not captured",
            None,
        ))
    })?;

    let stderr_task = tokio::spawn(async move {
        let mut text = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut text).await;
        text
    });

    let timeout_ms = limits.timeout_ms.unwrap_or(30_000);
    tokio::time::timeout(Duration::from_millis(timeout_ms), async {
        let mut lines = BufReader::new(stdout).lines();
        let mut events = Vec::new();
        let mut result = None;

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?
        {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let runtime_line =
                serde_json::from_str::<ProviderRuntimeLine>(trimmed).map_err(|error| {
                    PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
                        "invalid_provider_ndjson",
                        format!("invalid provider ndjson: {error}"),
                        Some(trimmed),
                    ))
                })?;
            match runtime_line {
                ProviderRuntimeLine::Result { result: value } => {
                    result = Some(value);
                }
                other => {
                    if let Some(event) = other.into_stream_event() {
                        if let Some(live_events) = &live_events {
                            let _ = live_events.send(event.clone());
                        }
                        events.push(event);
                    }
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;
        let stderr = stderr_task.await.unwrap_or_default();
        if !status.success() {
            let summary = stderr.trim();
            return Err(PluginFrameworkError::runtime(
                ProviderRuntimeError::normalize(
                    "provider_runtime",
                    if summary.is_empty() {
                        "provider runtime exited with failure"
                    } else {
                        summary
                    },
                    None,
                ),
            ));
        }

        let result = result.ok_or_else(|| {
            PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
                "provider_runtime",
                "provider runtime ended without result line",
                None,
            ))
        })?;

        Ok(StreamingProviderOutput { events, result })
    })
    .await
    .map_err(|_| provider_timeout_error())?
}

fn spawn_worker_process(
    executable_path: &Path,
    limits: &PluginRuntimeLimits,
) -> FrameworkResult<ProviderWorkerProcess> {
    let mut command = Command::new(executable_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    apply_memory_limit(&mut command, limits.memory_bytes)?;

    let mut child = command
        .spawn()
        .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;
    let stdin = child.stdin.take().ok_or_else(|| {
        PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
            "provider_runtime",
            "provider worker stdin was not captured",
            None,
        ))
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
            "provider_runtime",
            "provider worker stdout was not captured",
            None,
        ))
    })?;

    Ok(ProviderWorkerProcess {
        child,
        stdin,
        stdout: BufReader::new(stdout).lines(),
    })
}

async fn write_worker_request(
    executable_path: &Path,
    stdin: &mut ChildStdin,
    request: &ProviderStdioRequest,
) -> FrameworkResult<()> {
    let mut payload = serde_json::to_vec(request)
        .map_err(|error| PluginFrameworkError::serialization(None, error.to_string()))?;
    payload.push(b'\n');
    stdin
        .write_all(&payload)
        .await
        .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))?;
    stdin
        .flush()
        .await
        .map_err(|error| PluginFrameworkError::io(Some(executable_path), error.to_string()))
}

fn parse_stdio_response_line(executable_path: &Path, line: &str) -> FrameworkResult<Value> {
    let envelope = serde_json::from_str::<ProviderStdioResponse>(line).map_err(|error| {
        PluginFrameworkError::serialization(Some(executable_path), error.to_string())
    })?;

    if envelope.ok {
        return Ok(envelope.result);
    }

    let error = envelope.error.unwrap_or_else(|| ProviderStdioError {
        kind: ProviderRuntimeErrorKind::ProviderInvalidResponse,
        message: "provider runtime execution failed".to_string(),
        provider_summary: None,
    });
    Err(PluginFrameworkError::runtime(ProviderRuntimeError {
        kind: error.kind,
        message: error.message,
        provider_summary: error.provider_summary,
    }))
}

fn provider_timeout_error() -> PluginFrameworkError {
    PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
        "invoke",
        "provider runtime timed out",
        None,
    ))
}

fn worker_ended_without_output_error() -> PluginFrameworkError {
    PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
        "provider_runtime",
        "provider worker ended without response line",
        None,
    ))
}

fn worker_ended_without_result_error() -> PluginFrameworkError {
    PluginFrameworkError::runtime(ProviderRuntimeError::normalize(
        "provider_runtime",
        "provider worker ended without result line",
        None,
    ))
}

fn parse_stdio_response(
    executable_path: &Path,
    stdout: &[u8],
    stderr: &[u8],
) -> FrameworkResult<Value> {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stdout.is_empty() {
        return Err(PluginFrameworkError::runtime(
            ProviderRuntimeError::normalize(
                "provider_runtime",
                if stderr.is_empty() {
                    "provider runtime returned empty output"
                } else {
                    stderr.as_str()
                },
                None,
            ),
        ));
    }

    let envelope = serde_json::from_str::<ProviderStdioResponse>(&stdout).map_err(|error| {
        PluginFrameworkError::serialization(Some(executable_path), error.to_string())
    })?;

    if envelope.ok {
        return Ok(envelope.result);
    }

    let error = envelope.error.unwrap_or_else(|| ProviderStdioError {
        kind: ProviderRuntimeErrorKind::ProviderInvalidResponse,
        message: if stderr.is_empty() {
            "provider runtime execution failed".to_string()
        } else {
            stderr.clone()
        },
        provider_summary: None,
    });
    Err(PluginFrameworkError::runtime(ProviderRuntimeError {
        kind: error.kind,
        message: error.message,
        provider_summary: error.provider_summary,
    }))
}

fn apply_memory_limit(command: &mut Command, memory_bytes: Option<u64>) -> FrameworkResult<()> {
    #[cfg(unix)]
    {
        if let Some(limit) = memory_bytes {
            unsafe {
                command.pre_exec(move || {
                    let limit = libc::rlimit {
                        rlim_cur: limit as libc::rlim_t,
                        rlim_max: limit as libc::rlim_t,
                    };
                    if libc::setrlimit(libc::RLIMIT_AS, &limit) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (command, memory_bytes);
    }

    Ok(())
}
