use super::*;

#[test]
fn compile_rejects_edge_that_targets_unknown_node() {
    let flow_id = Uuid::now_v7();
    let mut document = sample_document(flow_id);
    document["graph"]["edges"][0]["target"] = json!("missing-node");

    let error =
        FlowCompiler::compile(flow_id, "draft-1", &document, &compile_context()).unwrap_err();

    assert!(error.to_string().contains("missing-node"));
}

#[test]
fn compile_rejects_internal_public_output_keys() {
    let flow_id = Uuid::now_v7();
    let mut document = sample_document(flow_id);
    document["graph"]["nodes"][1]["outputs"] =
        json!([{ "key": "__trace", "title": "Trace", "valueType": "json" }]);

    let error =
        FlowCompiler::compile(flow_id, "draft-1", &document, &compile_context()).unwrap_err();

    assert!(format!("{error:#}").contains("__trace"));
}

#[test]
fn compile_collects_provider_compile_issues() {
    let flow_id = Uuid::now_v7();
    let mut document = sample_document(flow_id);
    document["graph"]["nodes"][1]["config"] = json!({
        "model_provider": {
            "provider_code": "fixture_provider",
            "model_id": "unknown-model"
        }
    });

    let plan = FlowCompiler::compile(flow_id, "draft-1", &document, &compile_context()).unwrap();

    assert_eq!(plan.compile_issues.len(), 1);
    assert_eq!(
        plan.compile_issues[0].code,
        CompileIssueCode::ModelNotAvailable
    );
}

#[test]
fn compile_uses_selected_instance_models_instead_of_provider_family_aggregate() {
    let flow_id = Uuid::now_v7();
    let mut context = compile_context();
    context.provider_families.insert(
        "fixture_provider".to_string(),
        FlowCompileProviderFamily {
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            is_ready: true,
            available_models: BTreeSet::from([
                "gpt-5.4-mini".to_string(),
                "other-model".to_string(),
            ]),
            allow_custom_models: false,
        },
    );
    context.provider_instances.insert(
        "provider-selected".to_string(),
        FlowCompileProviderInstance {
            provider_instance_id: "provider-selected".to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            is_ready: true,
            is_runnable: true,
            included_in_main: true,
            available_models: BTreeSet::from(["other-model".to_string()]),
            allow_custom_models: false,
        },
    );

    let plan =
        FlowCompiler::compile(flow_id, "draft-1", &sample_document(flow_id), &context).unwrap();

    assert!(plan
        .compile_issues
        .iter()
        .any(|issue| issue.code == CompileIssueCode::ModelNotAvailable));
}

#[test]
fn compile_failover_queue_routes_with_frozen_targets() {
    let flow_id = Uuid::now_v7();
    let mut context = compile_context();
    context.provider_instances.insert(
        "provider-backup".to_string(),
        FlowCompileProviderInstance {
            provider_instance_id: "provider-backup".to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            is_ready: true,
            is_runnable: true,
            included_in_main: true,
            available_models: BTreeSet::from(["backup-model".to_string()]),
            allow_custom_models: false,
        },
    );
    let mut document = sample_document(flow_id);
    document["graph"]["nodes"][1]["config"]["model_provider"] = json!({
        "routing_mode": "failover_queue",
        "queue_template_id": "queue-template-1",
        "queue_snapshot_id": "queue-snapshot-1",
        "queue_targets": [
            {
                "provider_instance_id": "provider-selected",
                "provider_code": "fixture_provider",
                "protocol": "openai_compatible",
                "upstream_model_id": "gpt-5.4-mini"
            },
            {
                "provider_instance_id": "provider-backup",
                "provider_code": "fixture_provider",
                "protocol": "openai_compatible",
                "upstream_model_id": "backup-model"
            }
        ]
    });

    let plan = FlowCompiler::compile(flow_id, "draft-1", &document, &context).unwrap();
    let plan_json = serde_json::to_value(&plan).unwrap();
    let routing = &plan_json["nodes"]["node-llm"]["llm_runtime"]["routing"];

    assert!(plan.compile_issues.is_empty(), "{:?}", plan.compile_issues);
    assert_eq!(routing["routing_mode"], json!("failover_queue"));
    assert_eq!(routing["queue_template_id"], json!("queue-template-1"));
    assert_eq!(routing["queue_snapshot_id"], json!("queue-snapshot-1"));
    assert_eq!(
        routing["queue_targets"][0]["upstream_model_id"],
        json!("gpt-5.4-mini")
    );
    assert_eq!(
        routing["queue_targets"][1]["provider_instance_id"],
        json!("provider-backup")
    );
}

#[test]
fn compile_collects_missing_provider_issue() {
    let flow_id = Uuid::now_v7();
    let mut document = sample_document(flow_id);
    document["graph"]["nodes"][1]["config"]["model_provider"]["provider_code"] = Value::Null;

    let plan = FlowCompiler::compile(flow_id, "draft-1", &document, &compile_context()).unwrap();

    assert!(plan
        .compile_issues
        .iter()
        .any(|issue| issue.code == CompileIssueCode::MissingProviderInstance));
}

#[test]
fn compile_rejects_ambiguous_stable_provider_model_binding() {
    let flow_id = Uuid::now_v7();
    let mut context = compile_context();
    context.provider_instances.insert(
        "provider-recreated".to_string(),
        FlowCompileProviderInstance {
            provider_instance_id: "provider-recreated".to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            is_ready: true,
            is_runnable: true,
            included_in_main: true,
            available_models: BTreeSet::from(["gpt-5.4-mini".to_string()]),
            allow_custom_models: false,
        },
    );

    let plan =
        FlowCompiler::compile(flow_id, "draft-1", &sample_document(flow_id), &context).unwrap();

    assert!(plan.compile_issues.iter().any(|issue| {
        issue.code == CompileIssueCode::ProviderInstanceNotFound
            && issue.message.contains("ambiguous")
    }));
}

#[test]
fn compile_rejects_legacy_top_level_llm_config_shape() {
    let flow_id = Uuid::now_v7();
    let mut document = sample_document(flow_id);
    document["graph"]["nodes"][1]["config"] = json!({
        "provider_code": "fixture_provider",
        "source_instance_id": "provider-selected",
        "model": "gpt-5.4-mini"
    });

    let plan = FlowCompiler::compile(flow_id, "draft-1", &document, &compile_context()).unwrap();

    assert!(plan
        .compile_issues
        .iter()
        .any(|issue| issue.code == CompileIssueCode::MissingProviderInstance));
    assert!(plan
        .compile_issues
        .iter()
        .any(|issue| issue.code == CompileIssueCode::MissingModel));
}

#[test]
fn compile_plugin_node_emits_runtime_reference_from_registry_identity() {
    let flow_id = Uuid::now_v7();
    let plugin_context = plugin_compile_context();
    let installation_id = plugin_context
        .node_contributions
        .values()
        .next()
        .expect("plugin contribution should exist")
        .installation_id;
    let plan = FlowCompiler::compile(
        flow_id,
        "draft-1",
        &plugin_document(flow_id),
        &plugin_context,
    )
    .unwrap();

    let plan_json = serde_json::to_value(&plan).unwrap();

    assert_eq!(
        plan_json["nodes"]["node-plugin"]["node_type"],
        "plugin_node"
    );
    assert_eq!(
        plan_json["nodes"]["node-plugin"]["plugin_runtime"]["contribution_code"],
        "openai_prompt"
    );
    assert_eq!(
        plan_json["nodes"]["node-plugin"]["plugin_runtime"]["installation_id"],
        installation_id.to_string()
    );
    assert_eq!(
        plan_json["nodes"]["node-plugin"]["plugin_runtime"]["plugin_unique_identifier"],
        "prompt_pack"
    );
    assert_eq!(
        plan_json["nodes"]["node-plugin"]["plugin_runtime"]["package_id"],
        "prompt_pack@0.1.0"
    );
    assert_eq!(
        plan_json["nodes"]["node-plugin"]["plugin_runtime"]["contribution_checksum"],
        "sha256:contribution"
    );
    assert_eq!(
        plan_json["nodes"]["node-plugin"]["plugin_runtime"]["compiled_contribution_hash"],
        "sha256:compiled"
    );
    assert_eq!(
        plan_json["nodes"]["node-plugin"]["plugin_runtime"]["output_schema_snapshot"][0]["key"],
        "answer"
    );
}

#[test]
fn compile_plugin_node_reports_dependency_issue_when_registry_information_is_missing() {
    let flow_id = Uuid::now_v7();
    let plan = FlowCompiler::compile(
        flow_id,
        "draft-1",
        &plugin_document(flow_id),
        &compile_context(),
    )
    .unwrap();

    assert!(
        plan.compile_issues
            .iter()
            .any(|issue| issue.node_id == "node-plugin"),
        "expected a compile issue for the plugin node, got {:?}",
        plan.compile_issues
    );
}

#[test]
fn compile_plugin_node_rejects_legacy_contribution_schema_version() {
    let flow_id = Uuid::now_v7();
    let mut document = plugin_document(flow_id);
    document["graph"]["nodes"][1]["schema_version"] = json!("1flowbase.node-contribution/v1");

    let plan =
        FlowCompiler::compile(flow_id, "draft-1", &document, &plugin_compile_context()).unwrap();

    assert!(plan.compile_issues.iter().any(|issue| {
        issue.node_id == "node-plugin"
            && issue.code == CompileIssueCode::UnsupportedPluginContributionSchemaVersion
    }));
    assert!(plan.nodes["node-plugin"].plugin_runtime.is_none());
}

#[test]
fn compile_plugin_node_reports_issue_when_contribution_checksum_drifts() {
    let flow_id = Uuid::now_v7();
    let mut document = plugin_document(flow_id);
    document["graph"]["nodes"][1]["contribution_checksum"] = json!("sha256:changed");

    let plan =
        FlowCompiler::compile(flow_id, "draft-1", &document, &plugin_compile_context()).unwrap();

    assert!(plan.compile_issues.iter().any(|issue| {
        issue.node_id == "node-plugin"
            && issue.code == CompileIssueCode::PluginContributionChecksumMismatch
    }));
}

#[test]
fn compile_plugin_node_reports_issue_when_output_schema_snapshot_drifts() {
    let flow_id = Uuid::now_v7();
    let mut document = plugin_document(flow_id);
    document["graph"]["nodes"][1]["output_schema_snapshot"] = json!({
        "outputs": [{ "key": "changed", "title": "Changed", "valueType": "string" }]
    });

    let plan =
        FlowCompiler::compile(flow_id, "draft-1", &document, &plugin_compile_context()).unwrap();

    assert!(plan.compile_issues.iter().any(|issue| {
        issue.node_id == "node-plugin"
            && issue.code == CompileIssueCode::PluginContributionOutputSchemaMismatch
    }));
}
