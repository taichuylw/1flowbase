use super::*;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn spawn_binary_response_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buffer = vec![0_u8; 1024];
                let _ = stream.read(&mut buffer).await;
                let body = [0_u8, 1, 2, 3];
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.write_all(&body).await.unwrap();
            });
        }
    });

    (format!("http://{addr}/download"), handle)
}

fn http_request_flow_document(flow_id: Uuid, url: String) -> Value {
    json!({
        "schemaVersion": "1flowbase.flow/v2",
        "meta": {
            "flowId": flow_id.to_string(),
            "name": "HTTP Request Agent",
            "description": "",
            "tags": []
        },
        "graph": {
            "nodes": [
                {
                    "id": "node-start",
                    "type": "start",
                    "alias": "Start",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 0, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {},
                    "outputs": []
                },
                {
                    "id": "node-http",
                    "type": "http_request",
                    "alias": "HTTP Request",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 240, "y": 0 },
                    "configVersion": 1,
                    "config": {
                        "method": "GET",
                        "url": url,
                        "body_type": "none",
                        "timeout_ms": 30000,
                        "max_response_bytes": 1048576,
                        "verify_ssl": true
                    },
                    "bindings": {
                        "params": { "kind": "named_bindings", "value": [] },
                        "headers": { "kind": "named_bindings", "value": [] },
                        "body": { "kind": "templated_text", "value": "" },
                        "urlencoded": { "kind": "named_bindings", "value": [] },
                        "form_data": { "kind": "named_bindings", "value": [] }
                    },
                    "outputs": [
                        { "key": "body", "title": "HTTP 响应正文", "valueType": "string" },
                        { "key": "status_code", "title": "响应状态码", "valueType": "number" },
                        { "key": "headers", "title": "响应头列表 JSON", "valueType": "object" },
                        { "key": "files", "title": "HTTP 响应文件", "valueType": "Array[File]" }
                    ]
                }
            ],
            "edges": [
                {
                    "id": "edge-start-http",
                    "source": "node-start",
                    "target": "node-http",
                    "sourceHandle": null,
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                }
            ]
        },
        "editor": {
            "viewport": { "x": 0, "y": 0, "zoom": 1 },
            "annotations": [],
            "activeContainerPath": []
        }
    })
}

#[tokio::test]
async fn http_request_binary_response_persists_as_file_record() {
    let (url, server) = spawn_binary_response_server().await;
    let service = OrchestrationRuntimeService::for_tests_with_file_storage();
    let seeded = service.seed_application_with_flow("HTTP Agent").await;
    let started = service
        .start_flow_debug_run(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: json!({}),
            document_snapshot: Some(http_request_flow_document(seeded.flow_id, url)),
            debug_session_id: None,
        })
        .await
        .unwrap();

    let completed = service
        .continue_flow_debug_run(ContinueFlowDebugRunCommand {
            application_id: seeded.application_id,
            flow_run_id: started.flow_run.id,
            workspace_id: Uuid::nil(),
        })
        .await
        .unwrap();

    let http_node = node_run(&completed, "node-http");
    let file = &http_node.output_payload["files"][0];

    assert_eq!(completed.flow_run.status, domain::FlowRunStatus::Succeeded);
    assert_eq!(http_node.status, domain::NodeRunStatus::Succeeded);
    assert!(file["id"].as_str().is_some());
    assert_eq!(file["filename"], json!("response.bin"));
    assert_eq!(file["mimetype"], json!("application/octet-stream"));
    assert_eq!(file["storage_id"], service.default_file_storage_id_json());
    assert!(file["path"].as_str().unwrap().contains("/orders/"));
    assert!(file["url"]
        .as_str()
        .unwrap()
        .starts_with("https://files.test/"));
    assert_ne!(file["storage_id"], json!("runtime-inline"));
    assert_ne!(file["meta"]["persisted"], json!(false));

    server.abort();
}
