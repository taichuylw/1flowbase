use control_plane::application_public_api::publications::ApplicationPublicationVersionRecord;
use domain::ApplicationRecord;
use serde_json::{json, Value};

use crate::openapi_docs::{
    DocsCatalog, DocsCatalogCategory, DocsCatalogCategoryOperations, DocsCatalogOperation,
};

const NATIVE_CATEGORY_ID: &str = "application-native-api";
const OPENAI_CATEGORY_ID: &str = "openai-compatible-api";
const ANTHROPIC_CATEGORY_ID: &str = "anthropic-compatible-api";

type RequestBodyBuilder = fn(&DocTextResolver) -> Value;
type ResponseBuilder = fn(&DocTextResolver) -> Value;

#[derive(Debug, Clone)]
pub struct ApplicationPublicDocsContext {
    pub application: ApplicationRecord,
    pub active_publication: Option<ApplicationPublicationVersionRecord>,
    pub locale: String,
}

#[derive(Debug, Clone, Copy)]
struct PublicOperation {
    id: &'static str,
    method: &'static str,
    path: &'static str,
    category_id: &'static str,
    doc_key: &'static str,
    request_body: Option<RequestBodyBuilder>,
    responses: ResponseBuilder,
    notes: OperationNotes,
}

#[derive(Debug, Clone, Copy)]
enum OperationNotes {
    CategoryLimitations,
    Text {
        zh_hans: &'static str,
        en_us: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocsLocale {
    EnUs,
    ZhHans,
}

fn docs_locale(context: &ApplicationPublicDocsContext) -> DocsLocale {
    match context.locale.as_str() {
        "zh_Hans" => DocsLocale::ZhHans,
        _ => DocsLocale::EnUs,
    }
}

pub fn build_application_public_docs_catalog(
    context: &ApplicationPublicDocsContext,
) -> DocsCatalog {
    let locale = docs_locale(context);
    let operations = public_operations();
    let categories = [
        (
            NATIVE_CATEGORY_ID,
            category_label(NATIVE_CATEGORY_ID, locale),
        ),
        (
            OPENAI_CATEGORY_ID,
            category_label(OPENAI_CATEGORY_ID, locale),
        ),
        (
            ANTHROPIC_CATEGORY_ID,
            category_label(ANTHROPIC_CATEGORY_ID, locale),
        ),
    ]
    .into_iter()
    .map(|(id, label)| DocsCatalogCategory {
        id: id.to_string(),
        label: label.unwrap_or(id).to_string(),
        operation_count: operations
            .iter()
            .filter(|operation| operation.category_id == id)
            .count(),
    })
    .collect();

    DocsCatalog {
        title: match locale {
            DocsLocale::ZhHans => "应用公开 API".to_string(),
            DocsLocale::EnUs => "Application Public API".to_string(),
        },
        version: "v1".to_string(),
        categories,
    }
}

pub fn build_application_public_docs_category_operations(
    context: &ApplicationPublicDocsContext,
    category_id: &str,
) -> Option<DocsCatalogCategoryOperations> {
    let locale = docs_locale(context);
    let operations = public_operations()
        .iter()
        .filter(|operation| operation.category_id == category_id)
        .map(|operation| to_catalog_operation(operation, locale))
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return None;
    }
    let label = category_label(category_id, locale)?;
    Some(DocsCatalogCategoryOperations {
        id: category_id.to_string(),
        label: label.to_string(),
        operations,
    })
}

pub fn build_application_public_docs_category_spec(
    context: &ApplicationPublicDocsContext,
    category_id: &str,
) -> Option<Value> {
    let operations = public_operations()
        .iter()
        .filter(|operation| operation.category_id == category_id)
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return None;
    }
    Some(openapi_spec(context, operations))
}

pub fn build_application_public_docs_operation_spec(
    context: &ApplicationPublicDocsContext,
    operation_id: &str,
) -> Option<Value> {
    public_operations()
        .iter()
        .find(|operation| operation.id == operation_id)
        .map(|operation| openapi_spec(context, vec![operation]))
}

#[derive(Debug, Clone, Copy)]
struct DocTextResolver {
    locale: DocsLocale,
}

impl DocTextResolver {
    fn new(locale: DocsLocale) -> Self {
        Self { locale }
    }

    fn operation_summary(&self, doc_key: &str) -> &'static str {
        match (doc_key, self.locale) {
            ("application_public_api.native.create_run", DocsLocale::ZhHans) => "创建原生公开运行",
            ("application_public_api.native.get_run", DocsLocale::ZhHans) => "获取原生公开运行",
            ("application_public_api.native.cancel_run", DocsLocale::ZhHans) => "取消原生公开运行",
            ("application_public_api.native.resume_run", DocsLocale::ZhHans) => "恢复原生公开运行",
            ("application_public_api.native.upload_file", DocsLocale::ZhHans) => "上传原生公开文件",
            ("application_public_api.native.list_models", DocsLocale::ZhHans) => {
                "拉取原生模型能力列表"
            }
            ("application_public_api.openai.chat_completion", DocsLocale::ZhHans) => {
                "创建 OpenAI 兼容聊天补全"
            }
            ("application_public_api.openai.response", DocsLocale::ZhHans) => {
                "创建 OpenAI 兼容响应"
            }
            ("application_public_api.openai.list_models", DocsLocale::ZhHans) => {
                "拉取 OpenAI 兼容模型列表"
            }
            ("application_public_api.anthropic.message", DocsLocale::ZhHans) => {
                "创建 Anthropic 兼容消息"
            }
            ("application_public_api.anthropic.count_message_tokens", DocsLocale::ZhHans) => {
                "统计 Anthropic 兼容消息输入 tokens"
            }
            ("application_public_api.native.create_run", DocsLocale::EnUs) => {
                "Create Native public run"
            }
            ("application_public_api.native.get_run", DocsLocale::EnUs) => "Get Native public run",
            ("application_public_api.native.cancel_run", DocsLocale::EnUs) => {
                "Cancel Native public run"
            }
            ("application_public_api.native.resume_run", DocsLocale::EnUs) => {
                "Resume Native public run"
            }
            ("application_public_api.native.upload_file", DocsLocale::EnUs) => {
                "Upload Native public file"
            }
            ("application_public_api.native.list_models", DocsLocale::EnUs) => {
                "List Native model capabilities"
            }
            ("application_public_api.openai.chat_completion", DocsLocale::EnUs) => {
                "Create OpenAI-compatible chat completion"
            }
            ("application_public_api.openai.response", DocsLocale::EnUs) => {
                "Create OpenAI-compatible response"
            }
            ("application_public_api.openai.list_models", DocsLocale::EnUs) => {
                "List OpenAI-compatible models"
            }
            ("application_public_api.anthropic.message", DocsLocale::EnUs) => {
                "Create Anthropic-compatible message"
            }
            ("application_public_api.anthropic.count_message_tokens", DocsLocale::EnUs) => {
                "Count Anthropic-compatible message input tokens"
            }
            _ => "Public API operation",
        }
    }

    fn operation_description(&self, doc_key: &str) -> &'static str {
        match (doc_key, self.locale) {
            ("application_public_api.native.create_run", DocsLocale::ZhHans) => {
                "基于当前应用的活跃发布版本创建一次运行。"
            }
            ("application_public_api.native.get_run", DocsLocale::ZhHans) => {
                "读取由当前应用 API 密钥创建的公开运行。"
            }
            ("application_public_api.native.cancel_run", DocsLocale::ZhHans) => {
                "取消由当前应用 API 密钥创建的公开运行。"
            }
            ("application_public_api.native.resume_run", DocsLocale::ZhHans) => {
                "完成原生公开运行中等待回调的任务。"
            }
            ("application_public_api.native.upload_file", DocsLocale::ZhHans) => {
                "上传可供原生公开运行使用的文件。"
            }
            ("application_public_api.native.list_models", DocsLocale::ZhHans) => {
                "读取当前应用活跃发布版本中起始节点声明的模型能力目录。"
            }
            ("application_public_api.openai.chat_completion", DocsLocale::ZhHans) => {
                "将 OpenAI Chat Completions 请求适配为原生公开运行。"
            }
            ("application_public_api.openai.response", DocsLocale::ZhHans) => {
                "将 OpenAI Responses 请求适配为原生公开运行，previous_response_id 会解析回原生公开运行。"
            }
            ("application_public_api.openai.list_models", DocsLocale::ZhHans) => {
                "读取当前应用活跃发布版本中起始节点暴露的 OpenAI 兼容模型列表。"
            }
            ("application_public_api.anthropic.message", DocsLocale::ZhHans) => {
                "将 Anthropic Messages 请求适配为原生公开运行。"
            }
            ("application_public_api.anthropic.count_message_tokens", DocsLocale::ZhHans) => {
                "校验应用 API 密钥并返回 Anthropic Messages 请求的输入 token 估算值，不创建原生公开运行。"
            }
            ("application_public_api.native.create_run", DocsLocale::EnUs) => {
                "Creates a run against the active published application version."
            }
            ("application_public_api.native.get_run", DocsLocale::EnUs) => {
                "Reads a public run created by this application API key."
            }
            ("application_public_api.native.cancel_run", DocsLocale::EnUs) => {
                "Cancels a public run created by this application API key."
            }
            ("application_public_api.native.resume_run", DocsLocale::EnUs) => {
                "Completes a waiting callback task for a Native public run."
            }
            ("application_public_api.native.upload_file", DocsLocale::EnUs) => {
                "Uploads a file for use by Native public runs."
            }
            ("application_public_api.native.list_models", DocsLocale::EnUs) => {
                "Lists model capabilities declared by the active published application's start node."
            }
            ("application_public_api.openai.chat_completion", DocsLocale::EnUs) => {
                "Adapts an OpenAI Chat Completions request to a Native public run."
            }
            ("application_public_api.openai.response", DocsLocale::EnUs) => {
                "Adapts an OpenAI Responses request to a Native public run and resolves previous_response_id back to a Native public run."
            }
            ("application_public_api.openai.list_models", DocsLocale::EnUs) => {
                "Lists OpenAI-compatible models exposed by the active published application's start node."
            }
            ("application_public_api.anthropic.message", DocsLocale::EnUs) => {
                "Adapts an Anthropic Messages request to a Native public run."
            }
            ("application_public_api.anthropic.count_message_tokens", DocsLocale::EnUs) => {
                "Authenticates the application API key and returns an input token estimate for an Anthropic Messages request without creating a Native public run."
            }
            _ => "Public API operation.",
        }
    }

    fn field_description(&self, key: &str) -> &'static str {
        match (key, self.locale) {
            ("application_public_api.native.create_run.request.query", DocsLocale::ZhHans) => {
                "用户输入，会映射到当前应用发布配置中的 query target。"
            }
            ("application_public_api.native.create_run.request.model", DocsLocale::ZhHans) => {
                "可选模型标识，会通过应用 API 映射传入运行。"
            }
            ("application_public_api.native.create_run.request.system", DocsLocale::ZhHans) => {
                "系统指令上下文，会与历史中的旧 system 消息合并为运行级 system。"
            }
            ("application_public_api.native.create_run.request.inputs", DocsLocale::ZhHans) => {
                "附加输入对象，会通过应用 API 映射写入目标节点。"
            }
            ("application_public_api.native.create_run.request.history", DocsLocale::ZhHans) => {
                "可供发布运行使用的 user / assistant / tool 会话历史。"
            }
            (
                "application_public_api.native.create_run.request.attachments",
                DocsLocale::ZhHans,
            ) => "可供发布运行使用的文件或外部资源。",
            (
                "application_public_api.native.create_run.request.attachments.value",
                DocsLocale::ZhHans,
            ) => "当前附件来源对应的值。",
            (
                "application_public_api.native.create_run.request.conversation",
                DocsLocale::ZhHans,
            ) => "外部会话元数据，例如用户或会话 ID。",
            ("application_public_api.native.create_run.request.expand_id", DocsLocale::ZhHans) => {
                "显式外部用户 ID。会写入公开运行日志，并优先于 conversation.user。"
            }
            ("application_public_api.native.create_run.request.title", DocsLocale::ZhHans) => {
                "运行标题。未传时默认使用用户输入，并截断到 255 个字符。"
            }
            (
                "application_public_api.native.create_run.request.stream_options",
                DocsLocale::ZhHans,
            ) => "流式返回选项，include_workflow_events=public 会启用公开工作流事件。",
            ("application_public_api.native.create_run.request.execution", DocsLocale::ZhHans) => {
                "发布运行的执行选项。支持 model_parameters.reasoning 作为运行时 reasoning 偏好。"
            }
            ("application_public_api.native.create_run.request.metadata", DocsLocale::ZhHans) => {
                "调用方元数据，会随公开运行持久化。"
            }
            (
                "application_public_api.anthropic.message.request.metadata",
                DocsLocale::ZhHans,
            ) => "附加元数据。metadata.expand_id 会映射为公开运行的外部用户标识。",
            (
                "application_public_api.native.resume_run.request.callback_task_id",
                DocsLocale::ZhHans,
            ) => "required_action 返回的回调任务 ID。",
            (
                "application_public_api.native.upload_file.request.file_table_id",
                DocsLocale::ZhHans,
            ) => "目标文件表 ID。",
            ("application_public_api.native.upload_file.request.file", DocsLocale::ZhHans) => {
                "要上传的文件二进制内容。"
            }
            (
                "application_public_api.openai.chat_completion.request.stream",
                DocsLocale::ZhHans,
            )
            | (
                "application_public_api.openai.response.request.stream",
                DocsLocale::ZhHans,
            )
            | ("application_public_api.anthropic.message.request.stream", DocsLocale::ZhHans) => {
                "true 会将请求映射为流式响应模式。"
            }
            ("application_public_api.native.create_run.request.query", DocsLocale::EnUs) => {
                "User input mapped to the published application's query target."
            }
            ("application_public_api.native.create_run.request.model", DocsLocale::EnUs) => {
                "Optional model identifier passed through application mapping."
            }
            ("application_public_api.native.create_run.request.system", DocsLocale::EnUs) => {
                "System instruction context merged with legacy system messages from history."
            }
            ("application_public_api.native.create_run.request.inputs", DocsLocale::EnUs) => {
                "Additional input values mapped by the application API mapping."
            }
            ("application_public_api.native.create_run.request.history", DocsLocale::EnUs) => {
                "User, assistant, and tool conversation history entries available to the published run."
            }
            ("application_public_api.native.create_run.request.attachments", DocsLocale::EnUs) => {
                "Files or external assets available to the published run."
            }
            (
                "application_public_api.native.create_run.request.attachments.value",
                DocsLocale::EnUs,
            ) => "Attachment value for the selected source.",
            ("application_public_api.native.create_run.request.conversation", DocsLocale::EnUs) => {
                "External conversation metadata such as user or conversation id."
            }
            ("application_public_api.native.create_run.request.expand_id", DocsLocale::EnUs) => {
                "Explicit external user id persisted on the public run and preferred over conversation.user."
            }
            ("application_public_api.native.create_run.request.title", DocsLocale::EnUs) => {
                "Run title. Defaults to the user query and is truncated to 255 characters."
            }
            (
                "application_public_api.native.create_run.request.stream_options",
                DocsLocale::EnUs,
            ) => {
                "Streaming options. include_workflow_events=public enables public workflow events."
            }
            ("application_public_api.native.create_run.request.execution", DocsLocale::EnUs) => {
                "Execution options for the published run. Supports model_parameters.reasoning as runtime reasoning preference."
            }
            ("application_public_api.native.create_run.request.metadata", DocsLocale::EnUs) => {
                "Caller metadata persisted with the public run."
            }
            (
                "application_public_api.anthropic.message.request.metadata",
                DocsLocale::EnUs,
            ) => "Additional metadata. metadata.expand_id maps to the public run external user id.",
            (
                "application_public_api.native.resume_run.request.callback_task_id",
                DocsLocale::EnUs,
            ) => "Callback task id returned in required_action.",
            (
                "application_public_api.native.upload_file.request.file_table_id",
                DocsLocale::EnUs,
            ) => "Target file table id.",
            ("application_public_api.native.upload_file.request.file", DocsLocale::EnUs) => {
                "File binary content."
            }
            ("application_public_api.openai.chat_completion.request.stream", DocsLocale::EnUs)
            | ("application_public_api.openai.response.request.stream", DocsLocale::EnUs)
            | ("application_public_api.anthropic.message.request.stream", DocsLocale::EnUs) => {
                "true maps the request to streaming response mode."
            }
            _ => "Field value.",
        }
    }

    fn response_description(&self, key: &str) -> &'static str {
        match (key, self.locale) {
            ("compatible_response", DocsLocale::ZhHans) => "兼容响应",
            ("compatible_token_count", DocsLocale::ZhHans) => "兼容输入 token 统计",
            ("compatible_model_list", DocsLocale::ZhHans) => "OpenAI 兼容模型列表",
            ("native_model_list", DocsLocale::ZhHans) => "原生模型能力列表",
            ("native_run", DocsLocale::ZhHans) => "原生运行",
            ("native_run_created", DocsLocale::ZhHans) => "原生运行已创建",
            ("file_uploaded", DocsLocale::ZhHans) => "文件已上传",
            ("invalid_request", DocsLocale::ZhHans) => "请求无效",
            ("invalid_application_api_key", DocsLocale::ZhHans) => "应用 API 密钥无效",
            ("forbidden", DocsLocale::ZhHans) => "无权访问该公开资源",
            ("not_found", DocsLocale::ZhHans) => "公开资源不存在",
            ("application_not_published_or_run_state_not_supported", DocsLocale::ZhHans) => {
                "应用未发布，或运行状态不支持当前操作"
            }
            ("compatible_response", DocsLocale::EnUs) => "Compatible response",
            ("compatible_token_count", DocsLocale::EnUs) => "Compatible input token count",
            ("compatible_model_list", DocsLocale::EnUs) => "OpenAI-compatible model list",
            ("native_model_list", DocsLocale::EnUs) => "Native model capability list",
            ("native_run", DocsLocale::EnUs) => "Native run",
            ("native_run_created", DocsLocale::EnUs) => "Native run created",
            ("file_uploaded", DocsLocale::EnUs) => "File uploaded",
            ("invalid_request", DocsLocale::EnUs) => "Invalid request",
            ("invalid_application_api_key", DocsLocale::EnUs) => "Invalid application API key",
            ("forbidden", DocsLocale::EnUs) => "Forbidden public resource",
            ("not_found", DocsLocale::EnUs) => "Public resource not found",
            ("application_not_published_or_run_state_not_supported", DocsLocale::EnUs) => {
                "Application is not published or run state is not supported"
            }
            _ => "Response",
        }
    }

    fn operation_notes(&self, operation: &PublicOperation) -> &'static str {
        match operation.notes {
            OperationNotes::CategoryLimitations => self.unsupported_notes(operation.category_id),
            OperationNotes::Text { zh_hans, en_us } => match self.locale {
                DocsLocale::ZhHans => zh_hans,
                DocsLocale::EnUs => en_us,
            },
        }
    }

    fn unsupported_notes(&self, category_id: &str) -> &'static str {
        match (category_id, self.locale) {
            (OPENAI_CATEGORY_ID, DocsLocale::ZhHans) => {
                "此 v1 兼容端点支持 Chat Completions 和 Responses 外层协议适配，tools、tool_choice、function_call 字段进入模型供应商调用，支持 tool 消息历史回传和返回 tool_calls；stream=true 时返回 text/event-stream，心跳文本为 heartbeat，推理增量会投影为对应协议事件。暂不支持音频输出、图片/文件内容和多模态生成。如需查看 required_action 或恢复运行，请使用原生 API。"
            }
            (ANTHROPIC_CATEGORY_ID, DocsLocale::ZhHans) => {
                "此 v1 兼容端点支持顶层 tools/tool_choice 进入模型供应商调用，支持 tool_use / tool_result 文本块历史回传；stream=true 时返回 text/event-stream，心跳文本为 heartbeat，推理增量映射为 thinking_delta。暂不支持 computer use、image/document blocks 和等待态恢复。如需查看 required_action 或恢复运行，请使用原生 API。"
            }
            (_, DocsLocale::ZhHans) => {
                "原生 API 支持查看 required_action 并恢复运行。response_mode=streaming 时返回 text/event-stream，并包含心跳、reasoning.delta、message.delta 和终态事件。公开路径不会包含 application_id。"
            }
            (OPENAI_CATEGORY_ID, DocsLocale::EnUs) => {
                "This v1 compatible endpoint adapts Chat Completions and Responses protocol shapes, forwards tools, tool_choice, and function_call fields into the model-provider invocation, forwards tool message history, and can return tool_calls. stream=true returns text/event-stream with heartbeat text heartbeat and protocol-shaped events. Unsupported: audio output, image/file content, and multimodal generation. Use the Native API for required_action inspection and resume."
            }
            (ANTHROPIC_CATEGORY_ID, DocsLocale::EnUs) => {
                "This v1 compatible endpoint forwards top-level tools/tool_choice into the model-provider invocation and supports tool_use/tool_result text block history. stream=true returns text/event-stream with heartbeat text heartbeat and reasoning deltas as thinking_delta. Unsupported: computer use, image/document blocks, and waiting-state resume. Use the Native API for required_action inspection and resume."
            }
            (_, DocsLocale::EnUs) => {
                "Native API supports required_action inspection and resume. response_mode=streaming returns text/event-stream with heartbeat, reasoning.delta, message.delta, and terminal run events. Public paths never include application_id."
            }
        }
    }
}

fn category_label(category_id: &str, locale: DocsLocale) -> Option<&'static str> {
    match (category_id, locale) {
        (NATIVE_CATEGORY_ID, DocsLocale::ZhHans) => Some("应用原生 API"),
        (OPENAI_CATEGORY_ID, DocsLocale::ZhHans) => Some("OpenAI 兼容 API"),
        (ANTHROPIC_CATEGORY_ID, DocsLocale::ZhHans) => Some("Anthropic 兼容 API"),
        (NATIVE_CATEGORY_ID, DocsLocale::EnUs) => Some("Application Native API"),
        (OPENAI_CATEGORY_ID, DocsLocale::EnUs) => Some("OpenAI Compatible API"),
        (ANTHROPIC_CATEGORY_ID, DocsLocale::EnUs) => Some("Anthropic Compatible API"),
        _ => None,
    }
}

fn to_catalog_operation(operation: &PublicOperation, locale: DocsLocale) -> DocsCatalogOperation {
    let docs = DocTextResolver::new(locale);
    let category_label =
        category_label(operation.category_id, locale).unwrap_or(operation.category_id);
    DocsCatalogOperation {
        id: operation.id.to_string(),
        method: operation.method.to_string(),
        path: operation.path.to_string(),
        summary: Some(docs.operation_summary(operation.doc_key).to_string()),
        description: Some(docs.operation_description(operation.doc_key).to_string()),
        tags: vec![category_label.to_string()],
        group: category_label.to_string(),
        deprecated: false,
    }
}

fn openapi_spec(
    context: &ApplicationPublicDocsContext,
    operations: Vec<&PublicOperation>,
) -> Value {
    let locale = docs_locale(context);
    let mut paths = serde_json::Map::new();
    for operation in operations {
        let method = operation.method.to_ascii_lowercase();
        let path_item = paths
            .entry(operation.path.to_string())
            .or_insert_with(|| json!({}));
        path_item
            .as_object_mut()
            .expect("path item object")
            .insert(method, operation_openapi_spec(&operation, locale));
    }

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": application_title(context),
            "version": publication_version(context),
            "description": application_description(context),
        },
        "servers": [{"url": "/"}],
        "paths": paths,
        "components": {
            "securitySchemes": {
                "applicationApiKey": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "Application API Key",
                    "description": security_scheme_description(locale)
                },
                "anthropicApplicationApiKey": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "x-api-key",
                    "description": security_scheme_description(locale)
                }
            }
        },
        "x-1flowbase-application": {
            "id": context.application.id,
            "name": context.application.name,
            "api_enabled": context
                .active_publication
                .as_ref()
                .map(|publication| publication.api_enabled)
                .unwrap_or(false),
            "active_publication_version": context
                .active_publication
                .as_ref()
                .map(|publication| publication.version_sequence),
            "mapping": context
                .active_publication
                .as_ref()
                .map(mapping_summary)
                .unwrap_or_else(|| json!({"status": "not_published"}))
        }
    })
}

fn operation_openapi_spec(operation: &PublicOperation, locale: DocsLocale) -> Value {
    let docs = DocTextResolver::new(locale);
    let mut spec = json!({
        "operationId": operation.id,
        "summary": docs.operation_summary(operation.doc_key),
        "description": format!("{}\n\n{}", docs.operation_description(operation.doc_key), docs.operation_notes(operation)),
        "tags": [category_label(operation.category_id, locale).unwrap_or(operation.category_id)],
        "responses": operation_responses(operation, &docs),
        "security": operation_security(operation.category_id)
    });
    let spec_object = spec.as_object_mut().expect("operation spec object");
    let parameters = operation_parameters(operation);
    if !parameters.is_empty() {
        spec_object.insert("parameters".to_string(), Value::Array(parameters));
    }
    if let Some(request_body) = operation_request_body(operation, &docs) {
        spec_object.insert("requestBody".to_string(), request_body);
    }
    spec
}

fn operation_parameters(operation: &PublicOperation) -> Vec<Value> {
    let mut parameters = Vec::new();
    if operation.path.contains("{run_id}") {
        parameters.push(json!({
            "name": "run_id",
            "in": "path",
            "required": true,
            "description": "Published run id",
            "schema": {
                "type": "string",
                "format": "uuid"
            }
        }));
    }
    parameters
}

fn operation_request_body(operation: &PublicOperation, docs: &DocTextResolver) -> Option<Value> {
    operation.request_body.map(|builder| builder(docs))
}

fn operation_responses(operation: &PublicOperation, docs: &DocTextResolver) -> Value {
    (operation.responses)(docs)
}

fn native_create_run_request_body(docs: &DocTextResolver) -> Value {
    json_request_body(
        native_create_run_schema(docs),
        json!({
            "query": "Summarize the incident",
            "expand_id": "external-user-1",
            "title": "Customer incident summary",
            "response_mode": "blocking",
            "inputs": {"priority": "high"},
            "execution": {
                "model_parameters": {
                    "reasoning": {
                        "enabled": true,
                        "effort": "high"
                    }
                }
            },
            "conversation": {"user": "external-user-1"},
            "attachments": [{"source": "upload_file_id", "value": "00000000-0000-0000-0000-000000000000"}]
        }),
    )
}

fn native_resume_run_request_body(docs: &DocTextResolver) -> Value {
    json_request_body(
        native_resume_run_schema(docs),
        json!({
            "callback_task_id": "00000000-0000-0000-0000-000000000000",
            "response_payload": {},
            "response_mode": "blocking"
        }),
    )
}

fn native_upload_file_request_body(docs: &DocTextResolver) -> Value {
    json!({
        "required": true,
        "content": {
            "multipart/form-data": {
                "schema": native_file_upload_schema(docs)
            }
        }
    })
}

fn openai_chat_completion_request_body(docs: &DocTextResolver) -> Value {
    json_request_body(
        openai_chat_completion_schema(docs),
        json!({
            "model": "provider/model",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": false
        }),
    )
}

fn openai_response_create_request_body(docs: &DocTextResolver) -> Value {
    json_request_body(
        openai_response_create_schema(docs),
        json!({
            "model": "provider/model",
            "input": "Hello",
            "previous_response_id": "resp_00000000-0000-0000-0000-000000000000",
            "stream": false
        }),
    )
}

fn anthropic_message_request_body(docs: &DocTextResolver) -> Value {
    json_request_body(
        anthropic_message_schema(docs),
        json!({
            "model": "provider/model",
            "max_tokens": 512,
            "messages": [{"role": "user", "content": "Hello"}],
            "metadata": {"expand_id": "external-user-1"},
            "stream": false
        }),
    )
}

fn anthropic_count_message_tokens_request_body(_docs: &DocTextResolver) -> Value {
    json_request_body(
        anthropic_count_tokens_schema(),
        json!({
            "model": "provider/model",
            "messages": [{"role": "user", "content": "Hello"}],
            "tools": [{
                "name": "lookup_order",
                "description": "Find an order",
                "input_schema": {"type": "object"}
            }]
        }),
    )
}

fn native_create_run_responses(docs: &DocTextResolver) -> Value {
    native_responses(docs, "201", true)
}

fn native_get_run_responses(docs: &DocTextResolver) -> Value {
    native_responses(docs, "200", false)
}

fn native_resume_run_responses(docs: &DocTextResolver) -> Value {
    native_responses(docs, "200", true)
}

fn native_model_list_responses(docs: &DocTextResolver) -> Value {
    json!({
        "200": json_response(
            docs.response_description("native_model_list"),
            native_model_list_response_schema()
        ),
        "401": json_response(docs.response_description("invalid_application_api_key"), native_error_body_schema()),
        "403": json_response(docs.response_description("forbidden"), native_error_body_schema()),
        "409": json_response(
            docs.response_description("application_not_published_or_run_state_not_supported"),
            native_error_body_schema()
        )
    })
}

fn native_responses(
    docs: &DocTextResolver,
    success_status: &'static str,
    supports_streaming: bool,
) -> Value {
    let mut responses = serde_json::Map::new();
    let success_schema = api_success_schema(native_run_response_schema());
    let success_response = if supports_streaming {
        json_and_event_stream_response(
            docs.response_description(if success_status == "201" {
                "native_run_created"
            } else {
                "native_run"
            }),
            success_schema,
            native_streaming_event_schema(),
        )
    } else {
        json_response(
            docs.response_description(if success_status == "201" {
                "native_run_created"
            } else {
                "native_run"
            }),
            success_schema,
        )
    };
    responses.insert(success_status.to_string(), success_response);
    responses.insert(
        "400".to_string(),
        json_response(
            docs.response_description("invalid_request"),
            native_error_body_schema(),
        ),
    );
    responses.insert(
        "401".to_string(),
        json_response(
            docs.response_description("invalid_application_api_key"),
            native_error_body_schema(),
        ),
    );
    responses.insert(
        "403".to_string(),
        json_response(
            docs.response_description("forbidden"),
            native_error_body_schema(),
        ),
    );
    responses.insert(
        "404".to_string(),
        json_response(
            docs.response_description("not_found"),
            native_error_body_schema(),
        ),
    );
    responses.insert(
        "409".to_string(),
        json_response(
            docs.response_description("application_not_published_or_run_state_not_supported"),
            native_error_body_schema(),
        ),
    );
    Value::Object(responses)
}

fn native_upload_responses(docs: &DocTextResolver) -> Value {
    json!({
        "201": json_response(
            docs.response_description("file_uploaded"),
            api_success_schema(uploaded_file_response_schema())
        ),
        "400": json_response(docs.response_description("invalid_request"), native_error_body_schema()),
        "401": json_response(docs.response_description("invalid_application_api_key"), native_error_body_schema())
    })
}

fn openai_responses(docs: &DocTextResolver) -> Value {
    json!({
        "200": json_and_event_stream_response(
            docs.response_description("compatible_response"),
            openai_chat_completion_response_schema(),
            openai_streaming_event_schema()
        ),
        "400": json_response(docs.response_description("invalid_request"), openai_error_body_schema()),
        "401": json_response(docs.response_description("invalid_application_api_key"), openai_error_body_schema()),
        "403": json_response(docs.response_description("forbidden"), openai_error_body_schema()),
        "409": json_response(
            docs.response_description("application_not_published_or_run_state_not_supported"),
            openai_error_body_schema()
        )
    })
}

fn openai_model_list_responses(docs: &DocTextResolver) -> Value {
    json!({
        "200": json_response(
            docs.response_description("compatible_model_list"),
            openai_model_list_response_schema()
        ),
        "401": json_response(docs.response_description("invalid_application_api_key"), openai_error_body_schema()),
        "403": json_response(docs.response_description("forbidden"), openai_error_body_schema()),
        "409": json_response(
            docs.response_description("application_not_published_or_run_state_not_supported"),
            openai_error_body_schema()
        )
    })
}

fn openai_response_responses(docs: &DocTextResolver) -> Value {
    json!({
        "200": json_and_event_stream_response(
            docs.response_description("compatible_response"),
            openai_response_response_schema(),
            openai_response_streaming_event_schema()
        ),
        "400": json_response(docs.response_description("invalid_request"), openai_error_body_schema()),
        "401": json_response(docs.response_description("invalid_application_api_key"), openai_error_body_schema()),
        "403": json_response(docs.response_description("forbidden"), openai_error_body_schema()),
        "409": json_response(
            docs.response_description("application_not_published_or_run_state_not_supported"),
            openai_error_body_schema()
        )
    })
}

fn anthropic_responses(docs: &DocTextResolver) -> Value {
    json!({
        "200": json_and_event_stream_response(
            docs.response_description("compatible_response"),
            anthropic_message_response_schema(),
            anthropic_streaming_event_schema()
        ),
        "400": json_response(docs.response_description("invalid_request"), anthropic_error_body_schema()),
        "401": json_response(docs.response_description("invalid_application_api_key"), anthropic_error_body_schema()),
        "403": json_response(docs.response_description("forbidden"), anthropic_error_body_schema()),
        "409": json_response(
            docs.response_description("application_not_published_or_run_state_not_supported"),
            anthropic_error_body_schema()
        )
    })
}

fn anthropic_count_tokens_responses(docs: &DocTextResolver) -> Value {
    json!({
        "200": json_response(
            docs.response_description("compatible_token_count"),
            anthropic_count_tokens_response_schema()
        ),
        "400": json_response(docs.response_description("invalid_request"), anthropic_error_body_schema()),
        "401": json_response(docs.response_description("invalid_application_api_key"), anthropic_error_body_schema())
    })
}

fn json_response(description: &'static str, schema: Value) -> Value {
    json!({
        "description": description,
        "content": {
            "application/json": {
                "schema": schema
            }
        }
    })
}

fn json_and_event_stream_response(
    description: &'static str,
    json_schema: Value,
    event_stream_schema: Value,
) -> Value {
    json!({
        "description": description,
        "content": {
            "application/json": {
                "schema": json_schema
            },
            "text/event-stream": {
                "schema": event_stream_schema
            }
        }
    })
}

fn json_request_body(schema: Value, example: Value) -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": schema,
                "example": example
            }
        }
    })
}

fn api_success_schema(data_schema: Value) -> Value {
    json!({
        "type": "object",
        "required": ["data"],
        "properties": {
            "data": data_schema,
            "meta": {
                "oneOf": [
                    {"type": "object", "additionalProperties": true},
                    {"type": "null"}
                ]
            }
        }
    })
}

fn native_run_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "application_id", "api_key_id", "publication_version_id", "status", "node_input_payload", "metadata", "created_at"],
        "properties": {
            "id": {"type": "string", "format": "uuid"},
            "application_id": {"type": "string", "format": "uuid"},
            "api_key_id": {"type": "string", "format": "uuid"},
            "publication_version_id": {"type": "string", "format": "uuid"},
            "status": {"type": "string"},
            "node_input_payload": {"type": "object", "additionalProperties": true},
            "metadata": {"type": "object", "additionalProperties": true},
            "answer": {"oneOf": [{"type": "string"}, {"type": "null"}]},
            "required_action": {"oneOf": [{"type": "object", "additionalProperties": true}, {"type": "null"}]},
            "tool_calls": {"oneOf": [{"type": "array", "items": tool_call_schema()}, {"type": "null"}]},
            "usage": {"oneOf": [{"type": "object", "additionalProperties": true}, {"type": "null"}]},
            "error": {"oneOf": [{"type": "object", "additionalProperties": true}, {"type": "null"}]},
            "created_at": {"type": "string", "format": "date-time"}
        }
    })
}

fn native_model_list_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["object", "data"],
        "properties": {
            "object": {"type": "string", "enum": ["list"]},
            "data": {
                "type": "array",
                "items": native_model_object_schema()
            }
        }
    })
}

fn native_model_object_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "capabilities"],
        "properties": {
            "id": {"type": "string"},
            "name": {"type": "string"},
            "context_window": {"type": "integer"},
            "max_context_window": {"type": "integer"},
            "max_output_tokens": {"type": "integer"},
            "auto_compact_token_limit": {"type": "integer"},
            "capabilities": model_capabilities_schema(),
            "reasoning": model_reasoning_schema()
        }
    })
}

fn model_capabilities_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "reasoning": {"type": "boolean"},
            "tool_call": {"type": "boolean"},
            "multimodal": {"type": "boolean"},
            "structured_output": {"type": "boolean"}
        }
    })
}

fn model_reasoning_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "default_effort": {"type": "string"},
            "supported_efforts": {
                "type": "array",
                "items": {"type": "string"}
            }
        }
    })
}

fn native_error_body_schema() -> Value {
    json!({
        "type": "object",
        "required": ["code", "message"],
        "properties": {
            "code": {"type": "string"},
            "message": {"type": "string"}
        }
    })
}

fn uploaded_file_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["storage_id", "record"],
        "properties": {
            "storage_id": {"type": "string"},
            "record": {"type": "object", "additionalProperties": true}
        }
    })
}

fn tool_call_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "type": {"type": "string"},
            "name": {"type": "string"},
            "arguments": {
                "oneOf": [
                    {"type": "object", "additionalProperties": true},
                    {"type": "array"},
                    {"type": "string"},
                    {"type": "number"},
                    {"type": "boolean"},
                    {"type": "null"}
                ]
            }
        },
        "additionalProperties": true
    })
}

fn openai_tool_call_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "type", "function"],
        "properties": {
            "id": {"type": "string"},
            "type": {"type": "string", "enum": ["function"]},
            "function": {
                "type": "object",
                "required": ["name", "arguments"],
                "properties": {
                    "name": {"type": "string"},
                    "arguments": {"type": "string"}
                }
            }
        }
    })
}

fn openai_tool_schema() -> Value {
    json!({
        "type": "object",
        "required": ["type", "function"],
        "properties": {
            "type": {"type": "string", "enum": ["function"]},
            "function": {
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {"type": "string"},
                    "description": {"type": "string"},
                    "parameters": {"type": "object", "additionalProperties": true}
                }
            }
        },
        "additionalProperties": true
    })
}

fn anthropic_tool_schema() -> Value {
    json!({
        "type": "object",
        "required": ["name"],
        "properties": {
            "name": {"type": "string"},
            "description": {"type": "string"},
            "input_schema": {"type": "object", "additionalProperties": true}
        },
        "additionalProperties": true
    })
}

fn openai_chat_completion_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "object", "created", "model", "choices", "usage"],
        "properties": {
            "id": {"type": "string"},
            "object": {"type": "string", "enum": ["chat.completion"]},
            "created": {"type": "integer"},
            "model": {"type": "string"},
            "choices": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["index", "message", "finish_reason"],
                    "properties": {
                        "index": {"type": "integer"},
                        "message": {
                            "type": "object",
                            "required": ["role", "content"],
                            "properties": {
                                "role": {"type": "string", "enum": ["assistant"]},
                                "content": {"oneOf": [{"type": "string"}, {"type": "null"}]},
                                "tool_calls": {
                                    "type": "array",
                                    "items": openai_tool_call_schema()
                                }
                            }
                        },
                        "finish_reason": {"type": "string", "enum": ["stop", "tool_calls"]}
                    }
                }
            },
            "usage": {
                "type": "object",
                "properties": {
                    "prompt_tokens": {"type": "integer"},
                    "completion_tokens": {"type": "integer"},
                    "total_tokens": {"type": "integer"}
                }
            }
        }
    })
}

fn openai_model_list_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["object", "data"],
        "properties": {
            "object": {"type": "string", "enum": ["list"]},
            "data": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["id", "object", "created", "owned_by"],
                    "properties": {
                        "id": {"type": "string"},
                        "object": {"type": "string", "enum": ["model"]},
                        "created": {"type": "integer"},
                        "owned_by": {"type": "string"},
                        "context_window": {"type": "integer"},
                        "max_context_window": {"type": "integer"},
                        "max_output_tokens": {"type": "integer"},
                        "auto_compact_token_limit": {"type": "integer"},
                        "capabilities": model_capabilities_schema(),
                        "reasoning": model_reasoning_schema(),
                        "limit": {
                            "type": "object",
                            "properties": {
                                "context": {"type": "integer"},
                                "input": {"type": "integer"},
                                "output": {"type": "integer"}
                            }
                        },
                        "name": {
                            "oneOf": [
                                {"type": "string"},
                                {"type": "null"}
                            ]
                        }
                    }
                }
            }
        }
    })
}

fn openai_response_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "object", "created_at", "status", "model", "output", "output_text", "usage"],
        "properties": {
            "id": {"type": "string"},
            "object": {"type": "string", "enum": ["response"]},
            "created_at": {"type": "integer"},
            "status": {"type": "string", "enum": ["completed"]},
            "model": {"type": "string"},
            "previous_response_id": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "null"}
                ]
            },
            "output": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["id", "type", "status", "role", "content"],
                    "properties": {
                        "id": {"type": "string"},
                        "type": {"type": "string", "enum": ["message"]},
                        "status": {"type": "string", "enum": ["completed"]},
                        "role": {"type": "string", "enum": ["assistant"]},
                        "content": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["type", "text", "annotations"],
                                "properties": {
                                    "type": {"type": "string", "enum": ["output_text"]},
                                    "text": {"type": "string"},
                                    "annotations": {"type": "array", "items": {"type": "object", "additionalProperties": true}}
                                }
                            }
                        }
                    }
                }
            },
            "output_text": {"type": "string"},
            "usage": {
                "type": "object",
                "properties": {
                    "input_tokens": {"type": "integer"},
                    "output_tokens": {"type": "integer"},
                    "total_tokens": {"type": "integer"}
                }
            }
        }
    })
}

fn openai_error_body_schema() -> Value {
    json!({
        "type": "object",
        "required": ["error"],
        "properties": {
            "error": {
                "type": "object",
                "required": ["message", "type", "code"],
                "properties": {
                    "message": {"type": "string"},
                    "type": {"type": "string"},
                    "param": {"oneOf": [{"type": "string"}, {"type": "null"}]},
                    "code": {"type": "string"}
                }
            }
        }
    })
}

fn anthropic_message_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "type", "role", "model", "content", "stop_reason", "usage"],
        "properties": {
            "id": {"type": "string"},
            "type": {"type": "string", "enum": ["message"]},
            "role": {"type": "string", "enum": ["assistant"]},
            "model": {"type": "string"},
            "content": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["type"],
                    "properties": {
                        "type": {"type": "string", "enum": ["text", "tool_use"]},
                        "text": {"type": "string"},
                        "id": {"type": "string"},
                        "name": {"type": "string"},
                        "input": {"type": "object", "additionalProperties": true}
                    }
                }
            },
            "stop_reason": {"type": "string", "enum": ["end_turn", "tool_use"]},
            "usage": {
                "type": "object",
                "properties": {
                    "input_tokens": {"type": "integer"},
                    "output_tokens": {"type": "integer"}
                }
            }
        }
    })
}

fn anthropic_error_body_schema() -> Value {
    json!({
        "type": "object",
        "required": ["type", "error"],
        "properties": {
            "type": {"type": "string", "enum": ["error"]},
            "error": {
                "type": "object",
                "required": ["type", "message"],
                "properties": {
                    "type": {"type": "string"},
                    "message": {"type": "string"}
                }
            }
        }
    })
}

fn anthropic_count_tokens_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["input_tokens"],
        "properties": {
            "input_tokens": {"type": "integer", "minimum": 1}
        }
    })
}

fn native_streaming_event_schema() -> Value {
    json!({
        "type": "string",
        "format": "event-stream",
        "description": "Server-Sent Events emitted when response_mode=streaming.",
        "x-1flowbase-heartbeat": true,
        "x-1flowbase-events": [
            "run.started",
            "reasoning.delta",
            "message.delta",
            "workflow.event",
            "required_action",
            "run.completed",
            "run.failed",
            "run.cancelled"
        ],
        "x-1flowbase-reasoning-delta": "event: reasoning.delta",
        "x-1flowbase-message-delta": "event: message.delta"
    })
}

fn openai_streaming_event_schema() -> Value {
    json!({
        "type": "string",
        "format": "event-stream",
        "description": "OpenAI-compatible chat completion chunks emitted when stream=true.",
        "x-1flowbase-heartbeat": {
            "interval_seconds": 10,
            "text": "heartbeat"
        },
        "x-1flowbase-reasoning-delta": "choices[0].delta.reasoning_content",
        "x-1flowbase-message-delta": "choices[0].delta.content",
        "x-1flowbase-terminal-data": "[DONE]"
    })
}

fn openai_response_streaming_event_schema() -> Value {
    json!({
        "type": "string",
        "format": "event-stream",
        "description": "OpenAI Responses-compatible events emitted when stream=true.",
        "x-1flowbase-heartbeat": {
            "interval_seconds": 10,
            "text": "heartbeat"
        },
        "x-1flowbase-created": "response.created",
        "x-1flowbase-message-delta": "response.output_text.delta",
        "x-1flowbase-reasoning-delta": "response.reasoning_text.delta",
        "x-1flowbase-terminal-events": [
            "response.completed",
            "response.failed"
        ]
    })
}

fn anthropic_streaming_event_schema() -> Value {
    json!({
        "type": "string",
        "format": "event-stream",
        "description": "Anthropic-compatible message stream events emitted when stream=true.",
        "x-1flowbase-heartbeat": {
            "interval_seconds": 10,
            "text": "heartbeat"
        },
        "x-1flowbase-reasoning-delta": {
            "type": "content_block_delta",
            "delta": {
                "type": "thinking_delta",
                "field": "thinking"
            }
        },
        "x-1flowbase-message-delta": {
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "field": "text"
            }
        }
    })
}

fn native_create_run_schema(docs: &DocTextResolver) -> Value {
    json!({
        "type": "object",
        "required": ["query"],
        "properties": {
            "query": {
                "type": "string",
                "description": docs.field_description("application_public_api.native.create_run.request.query")
            },
            "model": {
                "type": "string",
                "description": docs.field_description("application_public_api.native.create_run.request.model")
            },
            "system": {
                "type": "string",
                "description": docs.field_description("application_public_api.native.create_run.request.system")
            },
            "inputs": {
                "type": "object",
                "additionalProperties": true,
                "description": docs.field_description("application_public_api.native.create_run.request.inputs")
            },
            "history": {
                "type": "array",
                "items": {"type": "object", "additionalProperties": true},
                "description": docs.field_description("application_public_api.native.create_run.request.history")
            },
            "attachments": {
                "type": "array",
                "items": native_attachment_schema(docs),
                "description": docs.field_description("application_public_api.native.create_run.request.attachments")
            },
            "conversation": {
                "type": "object",
                "additionalProperties": true,
                "description": docs.field_description("application_public_api.native.create_run.request.conversation")
            },
            "expand_id": {
                "type": "string",
                "description": docs.field_description("application_public_api.native.create_run.request.expand_id")
            },
            "title": {
                "type": "string",
                "maxLength": 255,
                "description": docs.field_description("application_public_api.native.create_run.request.title")
            },
            "response_mode": {
                "type": "string",
                "enum": ["blocking", "streaming"],
                "default": "blocking"
            },
            "stream_options": {
                "type": "object",
                "additionalProperties": true,
                "description": docs.field_description("application_public_api.native.create_run.request.stream_options")
            },
            "execution": {
                "type": "object",
                "additionalProperties": true,
                "description": docs.field_description("application_public_api.native.create_run.request.execution"),
                "properties": {
                    "idempotency_key": {"type": "string"},
                    "model_parameters": {
                        "type": "object",
                        "properties": {
                            "reasoning": {
                                "type": "object",
                                "properties": {
                                    "enabled": {"type": "boolean"},
                                    "effort": {"type": "string", "enum": ["minimal", "low", "medium", "high", "xhigh"]},
                                    "budget_tokens": {"type": "integer", "minimum": 1}
                                }
                            }
                        }
                    }
                }
            },
            "metadata": {
                "type": "object",
                "additionalProperties": true,
                "description": docs.field_description("application_public_api.native.create_run.request.metadata")
            }
        }
    })
}

fn native_attachment_schema(docs: &DocTextResolver) -> Value {
    json!({
        "type": "object",
        "properties": {
            "source": {
                "type": "string",
                "enum": ["upload_file_id", "url", "base64"]
            },
            "value": {
                "type": "string",
                "description": docs.field_description("application_public_api.native.create_run.request.attachments.value")
            },
            "name": {"type": "string"},
            "mime_type": {"type": "string"},
            "metadata": {"type": "object", "additionalProperties": true}
        }
    })
}

fn native_resume_run_schema(docs: &DocTextResolver) -> Value {
    json!({
        "type": "object",
        "required": ["callback_task_id"],
        "properties": {
            "callback_task_id": {
                "type": "string",
                "format": "uuid",
                "description": docs.field_description("application_public_api.native.resume_run.request.callback_task_id")
            },
            "response_payload": {
                "type": "object",
                "additionalProperties": true,
                "default": {}
            },
            "response_mode": {
                "type": "string",
                "enum": ["blocking", "streaming"],
                "default": "blocking"
            }
        }
    })
}

fn native_file_upload_schema(docs: &DocTextResolver) -> Value {
    json!({
        "type": "object",
        "required": ["file_table_id", "file"],
        "properties": {
            "file_table_id": {
                "type": "string",
                "format": "uuid",
                "description": docs.field_description("application_public_api.native.upload_file.request.file_table_id")
            },
            "file": {
                "type": "string",
                "format": "binary",
                "description": docs.field_description("application_public_api.native.upload_file.request.file")
            }
        }
    })
}

fn openai_chat_completion_schema(docs: &DocTextResolver) -> Value {
    json!({
        "type": "object",
        "required": ["model", "messages"],
        "properties": {
            "model": {"type": "string"},
            "messages": {
                "type": "array",
                "minItems": 1,
                "items": {
                    "type": "object",
                    "required": ["role", "content"],
                    "properties": {
                        "role": {"type": "string", "enum": ["system", "user", "assistant", "tool"]},
                        "content": {
                            "oneOf": [
                                {"type": "string"},
                                {"type": "null"},
                                {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "type": {"type": "string", "enum": ["text"]},
                                            "text": {"type": "string"}
                                        }
                                    }
                                }
                            ]
                        },
                        "name": {"type": "string"},
                        "tool_call_id": {"type": "string"},
                        "tool_calls": {
                            "type": "array",
                            "items": openai_tool_call_schema()
                        }
                    }
                }
            },
            "stream": {
                "type": "boolean",
                "description": docs.field_description("application_public_api.openai.chat_completion.request.stream")
            },
            "user": {"type": "string"},
            "tools": {
                "type": "array",
                "items": openai_tool_schema()
            },
            "tool_choice": {
                "oneOf": [
                    {"type": "string", "enum": ["none", "auto", "required"]},
                    {"type": "object", "additionalProperties": true}
                ]
            },
            "function_call": {
                "oneOf": [
                    {"type": "string", "enum": ["none", "auto"]},
                    {"type": "object", "additionalProperties": true}
                ]
            },
            "metadata": {"type": "object", "additionalProperties": true}
        }
    })
}

fn openai_response_create_schema(docs: &DocTextResolver) -> Value {
    json!({
        "type": "object",
        "required": ["model", "input"],
        "properties": {
            "model": {"type": "string"},
            "input": {
                "oneOf": [
                    {"type": "string"},
                    {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "role": {"type": "string", "enum": ["system", "user", "assistant"]},
                                "content": {
                                    "oneOf": [
                                        {"type": "string"},
                                        {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "type": {"type": "string", "enum": ["input_text", "text"]},
                                                    "text": {"type": "string"}
                                                }
                                            }
                                        }
                                    ]
                                }
                            },
                            "additionalProperties": true
                        }
                    }
                ]
            },
            "instructions": {"type": "string"},
            "previous_response_id": {"type": "string"},
            "stream": {
                "type": "boolean",
                "description": docs.field_description("application_public_api.openai.response.request.stream")
            },
            "user": {"type": "string"},
            "tools": {
                "type": "array",
                "items": openai_tool_schema()
            },
            "tool_choice": {
                "oneOf": [
                    {"type": "string", "enum": ["none", "auto", "required"]},
                    {"type": "object", "additionalProperties": true}
                ]
            },
            "metadata": {"type": "object", "additionalProperties": true}
        }
    })
}

fn anthropic_message_schema(docs: &DocTextResolver) -> Value {
    json!({
        "type": "object",
        "required": ["model", "messages"],
        "properties": {
            "model": {"type": "string"},
            "max_tokens": {"type": "integer", "minimum": 1},
            "system": {"type": "string"},
            "messages": {
                "type": "array",
                "minItems": 1,
                "items": {
                    "type": "object",
                    "required": ["role", "content"],
                    "properties": {
                        "role": {"type": "string", "enum": ["user", "assistant"]},
                        "content": {
                            "oneOf": [
                                {"type": "string"},
                                {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "type": {"type": "string", "enum": ["text", "tool_use", "tool_result"]},
                                            "text": {"type": "string"},
                                            "id": {"type": "string"},
                                            "name": {"type": "string"},
                                            "input": {"type": "object", "additionalProperties": true},
                                            "tool_use_id": {"type": "string"},
                                            "is_error": {"type": "boolean"},
                                            "content": {
                                                "oneOf": [
                                                    {"type": "string"},
                                                    {
                                                        "type": "array",
                                                        "items": {"type": "object", "additionalProperties": true}
                                                    },
                                                    {"type": "object", "additionalProperties": true}
                                                ]
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    }
                }
            },
            "stream": {
                "type": "boolean",
                "description": docs.field_description("application_public_api.anthropic.message.request.stream")
            },
            "tools": {
                "type": "array",
                "items": anthropic_tool_schema()
            },
            "tool_choice": {"type": "object", "additionalProperties": true},
            "metadata": {
                "type": "object",
                "properties": {
                    "expand_id": {"type": "string"},
                    "trace_id": {"type": "string"}
                },
                "additionalProperties": true,
                "description": docs.field_description("application_public_api.anthropic.message.request.metadata")
            }
        }
    })
}

fn anthropic_count_tokens_schema() -> Value {
    let content_block_schema = anthropic_count_tokens_content_block_schema();
    let message_content_schema = json!({
        "oneOf": [
            {"type": "string"},
            {
                "type": "array",
                "items": content_block_schema
            }
        ]
    });
    let message_schema = json!({
        "type": "object",
        "required": ["role", "content"],
        "properties": {
            "role": {"type": "string", "enum": ["user", "assistant"]},
            "content": message_content_schema
        },
        "additionalProperties": true
    });
    json!({
        "type": "object",
        "required": ["model", "messages"],
        "properties": {
            "model": {"type": "string"},
            "system": {
                "oneOf": [
                    {"type": "string"},
                    {
                        "type": "array",
                        "items": {"type": "object", "additionalProperties": true}
                    }
                ]
            },
            "messages": {
                "type": "array",
                "minItems": 1,
                "items": message_schema
            },
            "tools": {
                "type": "array",
                "items": anthropic_tool_schema()
            },
            "tool_choice": {
                "oneOf": [
                    {"type": "string", "enum": ["auto", "any", "none"]},
                    {"type": "object", "additionalProperties": true}
                ]
            },
            "thinking": {"type": "object", "additionalProperties": true},
            "container": {"type": "object", "additionalProperties": true},
            "context_management": {"type": "object", "additionalProperties": true},
            "metadata": {"type": "object", "additionalProperties": true}
        },
        "additionalProperties": true
    })
}

fn anthropic_count_tokens_content_block_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": [
                    "text",
                    "tool_use",
                    "tool_result",
                    "thinking",
                    "redacted_thinking",
                    "image",
                    "document"
                ]
            },
            "text": {"type": "string"},
            "id": {"type": "string"},
            "name": {"type": "string"},
            "input": {"type": "object", "additionalProperties": true},
            "tool_use_id": {"type": "string"},
            "is_error": {"type": "boolean"},
            "thinking": {"type": "string"},
            "signature": {"type": "string"},
            "source": {"type": "object", "additionalProperties": true},
            "content": {
                "oneOf": [
                    {"type": "string"},
                    {
                        "type": "array",
                        "items": {"type": "object", "additionalProperties": true}
                    },
                    {"type": "object", "additionalProperties": true}
                ]
            }
        },
        "additionalProperties": true
    })
}

fn operation_security(category_id: &str) -> Value {
    if category_id == ANTHROPIC_CATEGORY_ID {
        return json!([
            {"applicationApiKey": []},
            {"anthropicApplicationApiKey": []}
        ]);
    }

    json!([{"applicationApiKey": []}])
}

fn application_description(context: &ApplicationPublicDocsContext) -> String {
    let locale = docs_locale(context);
    let publication = context
        .active_publication
        .as_ref()
        .map(|publication| match locale {
            DocsLocale::ZhHans => {
                if publication.api_enabled {
                    format!("当前启用的是发布版本 v{}。", publication.version_sequence)
                } else {
                    format!("当前发布版本 v{} 未启用。", publication.version_sequence)
                }
            }
            DocsLocale::EnUs => {
                format!(
                    "Active publication v{} is {}.",
                    publication.version_sequence,
                    if publication.api_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                )
            }
        })
        .unwrap_or_else(|| match locale {
            DocsLocale::ZhHans => "当前没有活跃的公开 API 发布版本。".to_string(),
            DocsLocale::EnUs => "No active public API publication exists.".to_string(),
        });
    match locale {
        DocsLocale::ZhHans => format!(
            "{} 的应用级公开 API 文档。{}公开路径由应用 API 密钥选择，不通过 application_id 选择。",
            context.application.name, publication
        ),
        DocsLocale::EnUs => format!(
            "Application-scoped public API docs for {}. {} Public paths are selected by application API key, not by application_id.",
            context.application.name, publication
        ),
    }
}

fn application_title(context: &ApplicationPublicDocsContext) -> String {
    match docs_locale(context) {
        DocsLocale::ZhHans => format!("{} 公开 API", context.application.name),
        DocsLocale::EnUs => format!("{} Public API", context.application.name),
    }
}

fn publication_version(context: &ApplicationPublicDocsContext) -> String {
    context
        .active_publication
        .as_ref()
        .map(|publication| format!("publication-v{}", publication.version_sequence))
        .unwrap_or_else(|| "unpublished".to_string())
}

fn mapping_summary(publication: &ApplicationPublicationVersionRecord) -> Value {
    json!({
        "query_target": publication.mapping_snapshot.input.query_target,
        "model_target": publication.mapping_snapshot.input.model_target,
        "inputs_target": publication.mapping_snapshot.input.inputs_target,
        "history_target": publication.mapping_snapshot.input.history_target,
        "attachments_target": publication.mapping_snapshot.input.attachments_target,
        "answer_selector": publication.mapping_snapshot.output.answer_selector,
        "usage_selector": publication.mapping_snapshot.output.usage_selector,
        "files_selector": publication.mapping_snapshot.output.files_selector,
        "error_selector": publication.mapping_snapshot.output.error_selector,
    })
}

fn security_scheme_description(locale: DocsLocale) -> &'static str {
    match locale {
        DocsLocale::ZhHans => "使用在当前应用 API 页签中创建的应用 API 密钥。",
        DocsLocale::EnUs => "Use an application API key created from this application API tab.",
    }
}

fn public_operations() -> &'static [PublicOperation] {
    PUBLIC_OPERATION_REGISTRY
}

static PUBLIC_OPERATION_REGISTRY: &[PublicOperation] = &[
    PublicOperation {
        id: "applicationNativeCreateRun",
        method: "POST",
        path: "/api/agent/v1/runs",
        category_id: NATIVE_CATEGORY_ID,
        doc_key: "application_public_api.native.create_run",
        request_body: Some(native_create_run_request_body),
        responses: native_create_run_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationNativeGetRun",
        method: "GET",
        path: "/api/agent/v1/runs/{run_id}",
        category_id: NATIVE_CATEGORY_ID,
        doc_key: "application_public_api.native.get_run",
        request_body: None,
        responses: native_get_run_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationNativeCancelRun",
        method: "POST",
        path: "/api/agent/v1/runs/{run_id}/cancel",
        category_id: NATIVE_CATEGORY_ID,
        doc_key: "application_public_api.native.cancel_run",
        request_body: None,
        responses: native_get_run_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationNativeResumeRun",
        method: "POST",
        path: "/api/agent/v1/runs/{run_id}/resume",
        category_id: NATIVE_CATEGORY_ID,
        doc_key: "application_public_api.native.resume_run",
        request_body: Some(native_resume_run_request_body),
        responses: native_resume_run_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationNativeUploadFile",
        method: "POST",
        path: "/api/agent/v1/files",
        category_id: NATIVE_CATEGORY_ID,
        doc_key: "application_public_api.native.upload_file",
        request_body: Some(native_upload_file_request_body),
        responses: native_upload_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationNativeListModels",
        method: "GET",
        path: "/api/agent/v1/models",
        category_id: NATIVE_CATEGORY_ID,
        doc_key: "application_public_api.native.list_models",
        request_body: None,
        responses: native_model_list_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationOpenAiCreateChatCompletion",
        method: "POST",
        path: "/v1/chat/completions",
        category_id: OPENAI_CATEGORY_ID,
        doc_key: "application_public_api.openai.chat_completion",
        request_body: Some(openai_chat_completion_request_body),
        responses: openai_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationOpenAiCreateResponse",
        method: "POST",
        path: "/v1/responses",
        category_id: OPENAI_CATEGORY_ID,
        doc_key: "application_public_api.openai.response",
        request_body: Some(openai_response_create_request_body),
        responses: openai_response_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationOpenAiListModels",
        method: "GET",
        path: "/v1/models",
        category_id: OPENAI_CATEGORY_ID,
        doc_key: "application_public_api.openai.list_models",
        request_body: None,
        responses: openai_model_list_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationAnthropicCreateMessage",
        method: "POST",
        path: "/v1/messages",
        category_id: ANTHROPIC_CATEGORY_ID,
        doc_key: "application_public_api.anthropic.message",
        request_body: Some(anthropic_message_request_body),
        responses: anthropic_responses,
        notes: OperationNotes::CategoryLimitations,
    },
    PublicOperation {
        id: "applicationAnthropicCountMessageTokens",
        method: "POST",
        path: "/v1/messages/count_tokens",
        category_id: ANTHROPIC_CATEGORY_ID,
        doc_key: "application_public_api.anthropic.count_message_tokens",
        request_body: Some(anthropic_count_message_tokens_request_body),
        responses: anthropic_count_tokens_responses,
        notes: OperationNotes::Text {
            zh_hans: "该端点用于 Claude Code 等客户端的输入预估请求；只返回兼容形状的 token 估算结果，不写入运行记录。",
            en_us: "This endpoint supports input estimation requests from clients such as Claude Code; it returns a compatible token estimate and does not persist a run.",
        },
    },
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use utoipa::OpenApi;

    #[test]
    fn public_operation_registry_declares_unique_documented_operations() {
        let docs = DocTextResolver::new(DocsLocale::EnUs);
        let mut ids = HashSet::new();
        let mut routes = HashSet::new();

        for operation in public_operations() {
            assert!(ids.insert(operation.id), "duplicate operation id");
            assert!(
                routes.insert((operation.method, operation.path)),
                "duplicate public route"
            );
            assert!(
                category_label(operation.category_id, DocsLocale::EnUs).is_some(),
                "unknown category {}",
                operation.category_id
            );
            assert!(
                !operation_responses(operation, &docs)
                    .as_object()
                    .expect("operation responses should be an object")
                    .is_empty(),
                "operation {} must declare responses",
                operation.id
            );
            if let Some(request_body) = operation_request_body(operation, &docs) {
                assert!(
                    request_body.as_object().is_some(),
                    "operation {} request body should be an object",
                    operation.id
                );
            }
        }
    }

    #[test]
    fn public_operation_registry_matches_global_openapi_paths() {
        let spec = serde_json::to_value(crate::openapi::ApiDoc::openapi())
            .expect("global openapi should serialize");
        let paths = spec["paths"]
            .as_object()
            .expect("global openapi paths should be an object");

        for operation in public_operations() {
            let Some(path_item) = paths.get(operation.path) else {
                panic!("global openapi missing path {}", operation.path);
            };
            let method = operation.method.to_ascii_lowercase();
            assert!(
                path_item.get(&method).is_some(),
                "global openapi missing {} {}",
                operation.method,
                operation.path
            );
        }
    }
}
