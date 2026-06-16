use super::packages::{build_official_provider_package, official_upload_public_key};
use super::*;

#[derive(Clone, Default)]
pub(super) struct InMemoryOfficialPluginSource;

#[derive(Clone, Default)]
pub(super) struct InMemoryOfficialAgentFlowTemplateSource;

#[async_trait]
impl OfficialPluginSourcePort for InMemoryOfficialPluginSource {
    async fn list_official_catalog(&self) -> anyhow::Result<OfficialPluginCatalogSnapshot> {
        let package_bytes = build_official_provider_package("0.2.0");
        Ok(OfficialPluginCatalogSnapshot {
            source: OfficialPluginCatalogSource {
                source_kind: "mirror_registry".to_string(),
                source_label: "镜像源".to_string(),
                registry_url: "https://mirror.example.com/official-registry.json".to_string(),
            },
            entries: vec![OfficialPluginSourceEntry {
                plugin_id: "1flowbase.openai_compatible".to_string(),
                plugin_type: "model_provider".to_string(),
                provider_code: "openai_compatible".to_string(),
                namespace: "plugin.openai_compatible".to_string(),
                protocol: "openai_compatible".to_string(),
                latest_version: "0.2.0".to_string(),
                icon: Some(
                    "https://raw.githubusercontent.com/taichuy/1flowbase-official-plugins/main/runtime-extensions/model-providers/openai_compatible/_assets/icon.svg"
                        .to_string(),
                ),
                selected_artifact: OfficialPluginArtifact {
                    os: "linux".to_string(),
                    arch: "amd64".to_string(),
                    libc: Some("musl".to_string()),
                    rust_target: "x86_64-unknown-linux-musl".to_string(),
                    download_url: "https://example.com/openai-compatible.1flowbasepkg"
                        .to_string(),
                    checksum: format!("sha256:{:x}", Sha256::digest(&package_bytes)),
                    signature_algorithm: None,
                    signing_key_id: None,
                },
                i18n_summary: OfficialPluginI18nSummary {
                    default_locale: "en_US".to_string(),
                    available_locales: vec!["en_US".to_string(), "zh_Hans".to_string()],
                    bundles: std::collections::BTreeMap::from([
                        (
                            "en_US".to_string(),
                            json!({
                                "plugin": {
                                    "label": "OpenAI Compatible",
                                    "description": "Official provider plugin"
                                },
                                "provider": {
                                    "label": "OpenAI Compatible"
                                }
                            }),
                        ),
                        (
                            "zh_Hans".to_string(),
                            json!({
                                "plugin": {
                                    "label": "OpenAI Compatible",
                                    "description": "官方 Provider 插件"
                                },
                                "provider": {
                                    "label": "OpenAI Compatible"
                                }
                            }),
                        ),
                    ]),
                },
                release_tag: "openai_compatible-v0.2.0".to_string(),
                trust_mode: "allow_unsigned".to_string(),
                help_url: Some(
                    "https://github.com/taichuy/1flowbase-official-plugins/tree/main/models/openai_compatible"
                        .to_string(),
                ),
                model_discovery_mode: "hybrid".to_string(),
            }],
        })
    }

    async fn download_plugin(
        &self,
        _entry: &OfficialPluginSourceEntry,
    ) -> anyhow::Result<DownloadedOfficialPluginPackage> {
        Ok(DownloadedOfficialPluginPackage {
            file_name: "openai_compatible-0.2.0.1flowbasepkg".to_string(),
            package_bytes: build_official_provider_package("0.2.0"),
        })
    }

    fn trusted_public_keys(&self) -> Vec<plugin_framework::TrustedPublicKey> {
        vec![official_upload_public_key()]
    }
}

#[async_trait]
impl OfficialAgentFlowTemplateSourcePort for InMemoryOfficialAgentFlowTemplateSource {
    async fn list_catalog_page(
        &self,
        _cursor: Option<String>,
    ) -> anyhow::Result<OfficialAgentFlowTemplateCatalogSnapshot> {
        let template = build_agent_flow_template_package();
        let template_bytes = serde_json::to_vec(&template)?;

        Ok(OfficialAgentFlowTemplateCatalogSnapshot {
            source: OfficialAgentFlowTemplateCatalogSource {
                source_kind: "official_registry".to_string(),
                source_label: "官方源".to_string(),
                index_url:
                    "https://raw.githubusercontent.com/taichuy/1flowbase-official-plugins/main/agent-flow/catalog/v1/index.json"
                        .to_string(),
            },
            page: OfficialAgentFlowTemplateCatalogPage {
                page: 1,
                page_size: 100,
                next_cursor: None,
            },
            entries: vec![OfficialAgentFlowTemplateCatalogEntry {
                workflow_id: "multimodal-mount-test".to_string(),
                schema_version: "1flowbase.application-template/v1".to_string(),
                application: template.application.clone(),
                template_url:
                    "https://raw.githubusercontent.com/taichuy/1flowbase-official-plugins/main/agent-flow/workflows/multimodal-mount-test/template.json"
                        .to_string(),
                template_sha256: format!("sha256:{:x}", Sha256::digest(&template_bytes)),
                updated_at: "2026-06-16T00:00:00.000Z".to_string(),
            }],
        })
    }

    async fn download_template(
        &self,
        workflow_id: &str,
    ) -> anyhow::Result<control_plane::flow::AgentFlowTemplatePackage> {
        if workflow_id != "multimodal-mount-test" {
            return Err(control_plane::errors::ControlPlaneError::NotFound(
                "official_agent_flow_template",
            )
            .into());
        }

        Ok(build_agent_flow_template_package())
    }
}

fn build_agent_flow_template_package() -> control_plane::flow::AgentFlowTemplatePackage {
    control_plane::flow::AgentFlowTemplatePackage {
        schema_version: "1flowbase.application-template/v1".to_string(),
        application: control_plane::flow::AgentFlowTemplateApplication {
            application_type: "agent_flow".to_string(),
            name: "多模态挂载测试".to_string(),
            description: "".to_string(),
            icon: Some("RobotOutlined".to_string()),
            icon_type: Some("iconfont".to_string()),
            icon_background: Some("#E6F7F2".to_string()),
        },
        flow_document: json!({
            "schemaVersion": domain::FLOW_SCHEMA_VERSION,
            "meta": {
                "flowId": "019eb647-bee3-7ae2-a89d-5c6bca7921ad",
                "name": "多模态挂载测试",
                "description": "",
                "tags": []
            },
            "graph": {
                "nodes": [
                    {
                        "id": "node-start",
                        "type": "start",
                        "alias": "Start",
                        "config": { "input_fields": [] },
                        "outputs": [],
                        "bindings": {},
                        "position": { "x": 0, "y": 0 },
                        "containerId": null,
                        "description": "",
                        "configVersion": 1
                    }
                ],
                "edges": []
            }
        }),
        dependencies: Vec::new(),
    }
}
