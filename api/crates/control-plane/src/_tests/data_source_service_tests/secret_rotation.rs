use super::*;

#[tokio::test]
async fn create_instance_extracts_secret_like_config_values_to_reference_boundary() {
    let repository = InMemoryDataSourceRepository::default();
    let runtime = StubDataSourceRuntime::ready();
    let service = DataSourceService::new(repository.clone(), runtime);
    let plaintext_token = "plain-token-from-config";

    let created = service
        .create_instance(CreateDataSourceInstanceCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            installation_id: installation_id(),
            source_code: "acme_hubspot_source".into(),
            display_name: "HubSpot".into(),
            config_json: json!({
                "base_url": "https://api.example.test",
                "access_token": plaintext_token,
            }),
            secret_json: json!({ "client_secret": "plain-secret-body" }),
        })
        .await
        .unwrap();

    let stored_config_text = created.instance.config_json.to_string();
    assert!(!stored_config_text.contains(plaintext_token));
    assert_eq!(
        created.instance.config_json["access_token"],
        json!({
            "secret_ref": created.instance.secret_ref.as_ref().unwrap(),
            "secret_version": created.instance.secret_version.unwrap(),
        })
    );
    assert!(created
        .instance
        .secret_ref
        .as_ref()
        .unwrap()
        .starts_with("secret://workspace/"));
    assert_eq!(created.instance.secret_version, Some(1));

    let stored_secret = repository.stored_secret_json(created.instance.id).await;
    assert_eq!(stored_secret["access_token"], plaintext_token);
    assert_eq!(stored_secret["client_secret"], "plain-secret-body");

    let audit_text = serde_json::to_string(&repository.audit_events().await).unwrap();
    assert!(!audit_text.contains(plaintext_token));
    assert!(!audit_text.contains("plain-secret-body"));
}

#[tokio::test]
async fn create_instance_extracts_generic_secret_bearing_value_shapes() {
    let repository = InMemoryDataSourceRepository::default();
    let runtime = StubDataSourceRuntime::ready();
    let service = DataSourceService::new(repository.clone(), runtime);
    let header_plaintext = "bearer-from-header-value";
    let credential_plaintext = "credential-value-secret";

    let created = service
        .create_instance(CreateDataSourceInstanceCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            installation_id: installation_id(),
            source_code: "acme_hubspot_source".into(),
            display_name: "HubSpot".into(),
            config_json: json!({
                "headers": [
                    { "name": "Authorization", "value": header_plaintext },
                    { "name": "X-Trace", "value": "not-secret" }
                ],
                "credentials": { "type": "api_key", "value": credential_plaintext }
            }),
            secret_json: json!({}),
        })
        .await
        .unwrap();

    let config_text = created.instance.config_json.to_string();
    assert!(!config_text.contains(header_plaintext));
    assert!(!config_text.contains(credential_plaintext));
    assert!(config_text.contains("not-secret"));
    assert_eq!(
        created.instance.config_json["headers"][0]["value"],
        json!({
            "secret_ref": created.instance.secret_ref.as_ref().unwrap(),
            "secret_version": created.instance.secret_version.unwrap(),
        })
    );
    assert_eq!(
        created.instance.config_json["headers"][1]["value"],
        json!("not-secret")
    );
    assert_eq!(
        created.instance.config_json["credentials"]["value"],
        json!({
            "secret_ref": created.instance.secret_ref.as_ref().unwrap(),
            "secret_version": created.instance.secret_version.unwrap(),
        })
    );

    let stored_secret = repository.stored_secret_json(created.instance.id).await;
    assert_eq!(
        stored_secret["__config_secret_values"]["/headers/0/value"],
        header_plaintext
    );
    assert_eq!(
        stored_secret["__config_secret_values"].get("/headers/1/value"),
        None
    );
    assert_eq!(
        stored_secret["__config_secret_values"]["/credentials/value"],
        credential_plaintext
    );
}

#[tokio::test]
async fn rotate_secret_preserves_config_marker_values_when_payload_is_partial() {
    let repository = InMemoryDataSourceRepository::default();
    let runtime = StubDataSourceRuntime::ready();
    let service = DataSourceService::new(repository.clone(), runtime);

    let created = service
        .create_instance(CreateDataSourceInstanceCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            installation_id: installation_id(),
            source_code: "acme_hubspot_source".into(),
            display_name: "HubSpot".into(),
            config_json: json!({
                "access_token": "config-token-secret",
                "headers": [
                    { "name": "Authorization", "value": "authorization-secret" },
                    { "name": "X-Trace", "value": "not-secret" }
                ]
            }),
            secret_json: json!({
                "client_secret": "initial-client-secret",
                "__config_secret_values": {
                    "/manual/marker": "explicit-marker"
                }
            }),
        })
        .await
        .unwrap();

    let rotated = service
        .rotate_secret(RotateDataSourceSecretCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            instance_id: created.instance.id,
            secret_json: json!({ "client_secret": "rotated-client-secret" }),
        })
        .await
        .unwrap();

    let stored_secret = repository.stored_secret_json(created.instance.id).await;
    assert_eq!(stored_secret["client_secret"], "rotated-client-secret");
    assert_eq!(
        stored_secret["__config_secret_values"]["/access_token"],
        "config-token-secret"
    );
    assert_eq!(
        stored_secret["__config_secret_values"]["/headers/0/value"],
        "authorization-secret"
    );
    assert_eq!(
        stored_secret["__config_secret_values"]["/manual/marker"],
        "explicit-marker"
    );
    assert_eq!(
        stored_secret["__config_secret_values"].get("/headers/1/value"),
        None
    );
    assert_eq!(
        rotated.instance.config_json["access_token"]["secret_version"],
        json!(2)
    );
    assert_eq!(
        rotated.instance.config_json["headers"][0]["value"]["secret_version"],
        json!(2)
    );
    assert_eq!(
        rotated.instance.config_json["headers"][1]["value"],
        json!("not-secret")
    );
}

#[tokio::test]
async fn rotate_secret_updates_version_and_audit_without_cleartext() {
    let repository = InMemoryDataSourceRepository::default();
    let runtime = StubDataSourceRuntime::ready();
    let service = DataSourceService::new(repository.clone(), runtime);
    let rotated_plaintext = "rotated-secret-value";

    let created = service
        .create_instance(CreateDataSourceInstanceCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            installation_id: installation_id(),
            source_code: "acme_hubspot_source".into(),
            display_name: "HubSpot".into(),
            config_json: json!({ "base_url": "https://api.example.test" }),
            secret_json: json!({ "client_secret": "initial-secret-value" }),
        })
        .await
        .unwrap();

    let rotated = service
        .rotate_secret(RotateDataSourceSecretCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            instance_id: created.instance.id,
            secret_json: json!({ "client_secret": rotated_plaintext }),
        })
        .await
        .unwrap();

    assert_eq!(rotated.instance.secret_ref, created.instance.secret_ref);
    assert_eq!(rotated.instance.secret_version, Some(2));
    assert_eq!(
        repository.stored_secret_json(created.instance.id).await["client_secret"],
        rotated_plaintext
    );

    let audit_events = repository.audit_events().await;
    assert!(audit_events
        .iter()
        .any(|event| event.event_code == "data_source.secret_rotated"
            && event.payload["secret_ref"] == rotated.instance.secret_ref.clone().unwrap()
            && event.payload["secret_version"] == json!(2)));
    let audit_text = serde_json::to_string(&audit_events).unwrap();
    assert!(!audit_text.contains(rotated_plaintext));
    assert!(!audit_text.contains("initial-secret-value"));
}

#[tokio::test]
async fn sequential_secret_rotation_increments_versions_without_read_write_race_entrypoint() {
    let repository = InMemoryDataSourceRepository::default();
    let runtime = StubDataSourceRuntime::ready();
    let service = DataSourceService::new(repository.clone(), runtime);

    let created = service
        .create_instance(CreateDataSourceInstanceCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            installation_id: installation_id(),
            source_code: "acme_hubspot_source".into(),
            display_name: "HubSpot".into(),
            config_json: json!({ "access_token": "initial-config-secret" }),
            secret_json: json!({ "client_secret": "initial-secret-value" }),
        })
        .await
        .unwrap();

    let rotated_once = service
        .rotate_secret(RotateDataSourceSecretCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            instance_id: created.instance.id,
            secret_json: json!({ "client_secret": "rotated-once" }),
        })
        .await
        .unwrap();
    let rotated_twice = service
        .rotate_secret(RotateDataSourceSecretCommand {
            actor_user_id: user_id(),
            workspace_id: workspace_id(),
            instance_id: created.instance.id,
            secret_json: json!({ "client_secret": "rotated-twice" }),
        })
        .await
        .unwrap();

    assert_eq!(rotated_once.instance.secret_version, Some(2));
    assert_eq!(rotated_twice.instance.secret_version, Some(3));
    let secret_record = repository
        .get_secret_record(created.instance.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(secret_record.secret_version, 3);
    assert_eq!(
        rotated_twice.instance.config_json["access_token"]["secret_version"],
        json!(secret_record.secret_version)
    );
}
