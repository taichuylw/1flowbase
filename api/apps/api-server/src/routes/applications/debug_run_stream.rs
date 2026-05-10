use std::{convert::Infallible, sync::Arc};

use axum::response::sse::Event;
use control_plane::ports::{RuntimeEventEnvelope, RuntimeEventStream};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use tokio::sync::mpsc;
use uuid::Uuid;

pub type DebugRunSseStream = tokio_stream::wrappers::ReceiverStream<Result<Event, Infallible>>;

#[derive(Debug, Serialize)]
pub struct RuntimeEventStreamEnvelopeResponse {
    pub event_id: String,
    pub run_id: String,
    pub node_run_id: Option<String>,
    pub event_type: String,
    pub sequence: i64,
    pub created_at: String,
    pub payload: serde_json::Value,
    pub delta_index: Option<i64>,
    pub content_type: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RuntimeEventReplayExpiredResponse {
    #[serde(rename = "type")]
    pub response_type: &'static str,
    pub event_id: Option<String>,
    pub run_id: String,
    pub from_sequence: Option<i64>,
    pub reason: &'static str,
}

fn to_runtime_event_stream_envelope_response(
    envelope: RuntimeEventEnvelope,
) -> RuntimeEventStreamEnvelopeResponse {
    RuntimeEventStreamEnvelopeResponse {
        event_id: envelope.event_id,
        run_id: envelope.run_id.to_string(),
        node_run_id: envelope.node_run_id.map(|value| value.to_string()),
        event_type: envelope.event_type,
        sequence: envelope.sequence,
        created_at: envelope
            .occurred_at
            .format(&Rfc3339)
            .unwrap_or_else(|_| envelope.occurred_at.to_string()),
        payload: envelope.payload,
        delta_index: envelope.delta_index,
        content_type: envelope.content_type,
        text: envelope.text,
    }
}

pub fn runtime_event_to_sse(envelope: RuntimeEventEnvelope) -> Result<Event, Infallible> {
    let event_id = envelope.event_id.clone();
    let event_type = envelope.event_type.clone();

    Ok(Event::default()
        .id(event_id)
        .event(event_type)
        .json_data(to_runtime_event_stream_envelope_response(envelope))
        .expect("runtime event envelope should serialize"))
}

fn to_replay_expired_response(
    run_id: Uuid,
    from_sequence: Option<i64>,
) -> RuntimeEventReplayExpiredResponse {
    RuntimeEventReplayExpiredResponse {
        response_type: "replay_expired",
        event_id: from_sequence.map(|sequence| format!("{run_id}:{sequence}")),
        run_id: run_id.to_string(),
        from_sequence,
        reason: "cursor_expired",
    }
}

pub fn replay_expired_to_sse(
    run_id: Uuid,
    from_sequence: Option<i64>,
) -> Result<Event, Infallible> {
    let payload = to_replay_expired_response(run_id, from_sequence);
    let mut event = Event::default().event("replay_expired");
    if let Some(event_id) = &payload.event_id {
        event = event.id(event_id.clone());
    }

    Ok(event
        .json_data(payload)
        .expect("replay_expired payload should serialize"))
}

fn is_terminal_runtime_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "flow_finished" | "flow_failed" | "flow_cancelled" | "waiting_human" | "waiting_callback"
    )
}

pub async fn send_runtime_event_stream(
    stream: Arc<dyn RuntimeEventStream>,
    run_id: Uuid,
    from_sequence: Option<i64>,
    sender: mpsc::Sender<Result<Event, Infallible>>,
) {
    let Ok(mut subscription) = stream.subscribe(run_id, from_sequence).await else {
        let _ = sender
            .send(replay_expired_to_sse(run_id, from_sequence))
            .await;
        return;
    };

    for event in subscription.replay {
        let is_terminal = is_terminal_runtime_event(&event.event_type);
        if sender.send(runtime_event_to_sse(event)).await.is_err() {
            return;
        }
        if is_terminal {
            return;
        }
    }

    while let Some(event) = subscription.live_events.recv().await {
        let is_terminal = is_terminal_runtime_event(&event.event_type);
        if sender.send(runtime_event_to_sse(event)).await.is_err() {
            return;
        }
        if is_terminal {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use control_plane::ports::{
        RuntimeEventDurability, RuntimeEventPayload, RuntimeEventSource, RuntimeEventStreamPolicy,
    };
    use serde_json::json;
    use tokio::time::{timeout, Duration};

    use crate::host_infrastructure::LocalRuntimeEventStream;

    fn runtime_event(event_type: &str) -> RuntimeEventPayload {
        RuntimeEventPayload {
            event_type: event_type.to_string(),
            source: RuntimeEventSource::Runtime,
            durability: RuntimeEventDurability::DurableRequired,
            persist_required: true,
            trace_visible: true,
            payload: json!({ "type": event_type }),
        }
    }

    #[test]
    fn replay_expired_response_includes_cursor_contract() {
        let run_id = Uuid::now_v7();
        let payload = to_replay_expired_response(run_id, Some(42));
        let expected_event_id = format!("{run_id}:42");

        assert_eq!(payload.response_type, "replay_expired");
        assert_eq!(
            payload.event_id.as_deref(),
            Some(expected_event_id.as_str())
        );
        assert_eq!(payload.run_id, run_id.to_string());
        assert_eq!(payload.from_sequence, Some(42));
        assert_eq!(payload.reason, "cursor_expired");
    }

    #[tokio::test]
    async fn send_runtime_event_stream_returns_after_terminal_event() {
        let stream = Arc::new(LocalRuntimeEventStream::new());
        let run_id = Uuid::now_v7();
        stream
            .open_run(run_id, RuntimeEventStreamPolicy::debug_default())
            .await
            .unwrap();
        let (sender, mut receiver) = mpsc::channel(8);

        tokio::spawn(send_runtime_event_stream(
            stream.clone(),
            run_id,
            None,
            sender,
        ));
        stream
            .append(run_id, runtime_event("flow_finished"))
            .await
            .unwrap();

        let _ = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("terminal event should be sent")
            .expect("terminal event should be available")
            .expect("sse event should be valid");

        let closed = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(
            matches!(closed, Ok(None)),
            "sender should close after terminal event"
        );
    }

    #[tokio::test]
    async fn send_runtime_event_stream_returns_after_flow_cancelled_terminal_event() {
        let stream = Arc::new(LocalRuntimeEventStream::new());
        let run_id = Uuid::now_v7();
        stream
            .open_run(run_id, RuntimeEventStreamPolicy::debug_default())
            .await
            .unwrap();
        let (sender, mut receiver) = mpsc::channel(8);

        tokio::spawn(send_runtime_event_stream(
            stream.clone(),
            run_id,
            None,
            sender,
        ));
        stream
            .append(run_id, runtime_event("flow_cancelled"))
            .await
            .unwrap();

        let _ = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("cancelled terminal event should be sent")
            .expect("cancelled terminal event should be available")
            .expect("sse event should be valid");

        let closed = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(
            matches!(closed, Ok(None)),
            "sender should close after flow_cancelled terminal event"
        );
    }
}
