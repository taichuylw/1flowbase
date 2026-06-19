use plugin_framework::provider_contract::ClientProtocolEnvelope;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientProtocolIngressPolicy {
    AnthropicMessages,
    DefaultDeny,
}

pub fn capture_client_protocol_envelope<I, N, V>(
    policy: ClientProtocolIngressPolicy,
    headers: I,
) -> Option<ClientProtocolEnvelope>
where
    I: IntoIterator<Item = (N, V)>,
    N: AsRef<str>,
    V: AsRef<str>,
{
    let policy_spec = match policy {
        ClientProtocolIngressPolicy::AnthropicMessages => anthropic_messages_policy(),
        ClientProtocolIngressPolicy::DefaultDeny => return None,
    };
    let mut captured = BTreeMap::new();

    for (name, value) in headers {
        let header_name = name.as_ref().trim().to_ascii_lowercase();
        if header_name.is_empty()
            || blocked_header(&header_name)
            || !policy_spec.allowed_headers.contains(&header_name.as_str())
        {
            continue;
        }
        let header_value = value.as_ref().trim();
        if header_value.is_empty() {
            continue;
        }
        captured.insert(header_name, header_value.to_string());
    }

    (!captured.is_empty()).then(|| ClientProtocolEnvelope {
        source_protocol: policy_spec.source_protocol.to_string(),
        policy: policy_spec.policy.to_string(),
        headers: captured,
    })
}

struct ClientProtocolPolicySpec {
    source_protocol: &'static str,
    policy: &'static str,
    allowed_headers: &'static [&'static str],
}

fn anthropic_messages_policy() -> ClientProtocolPolicySpec {
    ClientProtocolPolicySpec {
        source_protocol: "anthropic_messages",
        policy: "anthropic_messages_v1",
        allowed_headers: &[
            "anthropic-version",
            "anthropic-beta",
            "x-claude-code-session-id",
            "anthropic-client-name",
            "anthropic-client-version",
            "x-client-name",
            "x-client-version",
            "user-agent",
        ],
    }
}

fn blocked_header(name: &str) -> bool {
    matches!(
        name,
        "authorization"
            | "proxy-authorization"
            | "x-api-key"
            | "api-key"
            | "cookie"
            | "set-cookie"
            | "x-csrf-token"
            | "x-xsrf-token"
            | "csrf-token"
            | "host"
            | "content-length"
            | "connection"
            | "transfer-encoding"
            | "accept-encoding"
            | "te"
            | "trailer"
            | "upgrade"
            | "keep-alive"
    )
}
