//! The AI providers MDViewer can summarise with, and how to talk to each.
//!
//! Almost every provider speaks the OpenAI "chat completions" format, so one
//! client covers them all; Anthropic gets its own Messages API format. This
//! file only builds requests and reads responses; `http.rs` sends them.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// `POST /chat/completions` with a bearer token.
    #[default]
    OpenAi,
    /// Anthropic's `POST /messages`.
    Anthropic,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub format: Format,
    pub base_url: &'static str,
    /// Local servers and custom endpoints may work without a key.
    pub needs_key: bool,
    /// Runs on this Mac, so documents never leave it.
    pub local: bool,
    /// Where to get a key.
    pub key_url: Option<&'static str>,
    /// Model used until the user picks one.
    pub default_model: Option<&'static str>,
    /// Local and custom endpoints let the user change the address.
    pub editable_url: bool,
}

const fn cloud(
    id: &'static str,
    name: &'static str,
    format: Format,
    base_url: &'static str,
    key_url: &'static str,
    default_model: Option<&'static str>,
) -> Provider {
    Provider {
        id,
        name,
        format,
        base_url,
        needs_key: true,
        local: false,
        key_url: Some(key_url),
        default_model,
        editable_url: false,
    }
}

const fn local(id: &'static str, name: &'static str, base_url: &'static str) -> Provider {
    Provider {
        id,
        name,
        format: Format::OpenAi,
        base_url,
        needs_key: false,
        local: true,
        key_url: None,
        default_model: None,
        editable_url: true,
    }
}

pub const CUSTOM: &str = "custom";

pub static PROVIDERS: &[Provider] = &[
    cloud(
        "openrouter",
        "OpenRouter",
        Format::OpenAi,
        "https://openrouter.ai/api/v1",
        "https://openrouter.ai/settings/keys",
        None,
    ),
    cloud(
        "openai",
        "OpenAI",
        Format::OpenAi,
        "https://api.openai.com/v1",
        "https://platform.openai.com/api-keys",
        None,
    ),
    cloud(
        "anthropic",
        "Anthropic",
        Format::Anthropic,
        "https://api.anthropic.com/v1",
        "https://platform.claude.com/settings/keys",
        Some("claude-opus-5-5"),
    ),
    cloud(
        "google",
        "Google Gemini",
        Format::OpenAi,
        "https://generativelanguage.googleapis.com/v1beta/openai",
        "https://aistudio.google.com/apikey",
        None,
    ),
    cloud(
        "xai",
        "xAI Grok",
        Format::OpenAi,
        "https://api.x.ai/v1",
        "https://console.x.ai",
        None,
    ),
    cloud(
        "mistral",
        "Mistral",
        Format::OpenAi,
        "https://api.mistral.ai/v1",
        "https://console.mistral.ai/api-keys",
        None,
    ),
    cloud(
        "deepseek",
        "DeepSeek",
        Format::OpenAi,
        "https://api.deepseek.com/v1",
        "https://platform.deepseek.com/api_keys",
        None,
    ),
    cloud(
        "groq",
        "Groq",
        Format::OpenAi,
        "https://api.groq.com/openai/v1",
        "https://console.groq.com/keys",
        None,
    ),
    local("ollama", "Ollama", "http://localhost:11434/v1"),
    local("lmstudio", "LM Studio", "http://localhost:1234/v1"),
    Provider {
        id: CUSTOM,
        name: "Custom provider",
        format: Format::OpenAi,
        base_url: "",
        needs_key: false,
        local: false,
        key_url: None,
        default_model: None,
        editable_url: true,
    },
];

pub fn find(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// Everything needed to call a provider: its fixed traits plus what the user
/// configured (model, address, key).
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub format: Format,
    pub base_url: String,
    pub key: Option<String>,
    pub model: String,
    /// OpenRouter asks apps to identify themselves.
    pub openrouter: bool,
}

#[derive(Debug, PartialEq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: Option<Vec<u8>>,
}

const ANTHROPIC_VERSION: &str = "2026-10-21";
/// Room for the summary, plus thinking on models that always think.
const ANTHROPIC_MAX_TOKENS: u32 = 16_000;

impl Endpoint {
    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.base_url.trim_end_matches('/'))
    }

    fn auth_headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = Vec::new();
        match (self.format, &self.key) {
            (Format::Anthropic, key) => {
                if let Some(key) = key {
                    headers.push(("x-api-key", key.clone()));
                }
                headers.push(("anthropic-version", ANTHROPIC_VERSION.into()));
            }
            (Format::OpenAi, Some(key)) => headers.push(("Authorization", format!("Bearer {key}"))),
            (Format::OpenAi, None) => {}
        }
        if self.openrouter {
            headers.push((
                "HTTP-Referer",
                "https://github.com/mmmikael/mdviewer".into(),
            ));
            headers.push(("X-Title", "MDViewer".into()));
        }
        headers
    }

    /// A streamed chat request with one system prompt and one user message.
    pub fn chat_request(&self, system: &str, user: &str) -> HttpRequest {
        let body = match self.format {
            // No token limit or temperature: providers disagree on their names
            // and on which models accept them; the prompt asks for brevity.
            Format::OpenAi => json!({
                "model": self.model,
                "stream": true,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user },
                ],
            }),
            Format::Anthropic => json!({
                "model": self.model,
                "max_tokens": ANTHROPIC_MAX_TOKENS,
                "stream": true,
                "system": system,
                "messages": [{ "role": "user", "content": user }],
            }),
        };
        let path = match self.format {
            Format::OpenAi => "chat/completions",
            Format::Anthropic => "messages",
        };
        let mut headers = self.auth_headers();
        headers.push(("Content-Type", "application/json".into()));
        headers.push(("Accept", "text/event-stream".into()));
        HttpRequest {
            method: "POST",
            url: self.url(path),
            headers,
            body: Some(body.to_string().into_bytes()),
        }
    }

    pub fn models_request(&self) -> HttpRequest {
        let path = match self.format {
            Format::OpenAi => "models",
            Format::Anthropic => "models?limit=1000",
        };
        HttpRequest {
            method: "GET",
            url: self.url(path),
            headers: self.auth_headers(),
            body: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: Option<String>,
    /// Context window in tokens, when the provider says.
    pub context_tokens: Option<u64>,
    /// US dollars per token, when the provider says (OpenRouter).
    pub prompt_price: Option<f64>,
    pub completion_price: Option<f64>,
}

/// Model ids that can't write text (embeddings, speech, images, …), which
/// OpenAI-style model lists mix in with chat models.
fn is_chat_model(id: &str) -> bool {
    const NOT_CHAT: &[&str] = &[
        "embed",
        "tts",
        "whisper",
        "dall-e",
        "moderation",
        "transcribe",
        "realtime",
        "image",
        "audio",
        "davinci",
        "babbage",
        "sora",
        "rerank",
    ];
    let id = id.to_ascii_lowercase();
    !NOT_CHAT.iter().any(|word| id.contains(word))
}

fn price(value: &Value) -> Option<f64> {
    match value {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
    .filter(|p: &f64| *p >= 0.0)
}

/// Reads a `GET /models` response from either format.
pub fn parse_models(body: &[u8]) -> Result<Vec<ModelInfo>, String> {
    let value: Value =
        serde_json::from_slice(body).map_err(|_| "The provider sent an unexpected model list.")?;
    // OpenAI style is `{ data: [...] }`; Ollama's native style is `{ models: [...] }`.
    let list = value
        .get("data")
        .or_else(|| value.get("models"))
        .and_then(Value::as_array)
        .ok_or("The provider sent an unexpected model list.")?;
    let mut models: Vec<ModelInfo> = list
        .iter()
        .filter_map(|m| {
            let id = m.get("id").and_then(Value::as_str)?;
            // Gemini lists `models/gemini-…` but chats with the bare id.
            let id = id.strip_prefix("models/").unwrap_or(id).to_string();
            let name = m
                .get("name")
                .or_else(|| m.get("display_name"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let context_tokens = m
                .get("context_length")
                .or_else(|| m.get("max_input_tokens"))
                .and_then(Value::as_u64);
            let pricing = m.get("pricing");
            Some(ModelInfo {
                id,
                name,
                context_tokens,
                prompt_price: pricing.and_then(|p| price(p.get("prompt")?)),
                completion_price: pricing.and_then(|p| price(p.get("completion")?)),
            })
        })
        .filter(|m| is_chat_model(&m.id))
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    Ok(models)
}

/// What one server-sent event means for the summary being written.
#[derive(Debug, PartialEq)]
pub enum StreamItem {
    Text(String),
    /// The model stopped early or refused; shown after the text.
    Notice(String),
    Error(String),
    Done,
}

/// Interprets one SSE event (`event:` name and `data:` payload) of a streamed reply.
pub fn parse_event(format: Format, event: Option<&str>, data: &str) -> Vec<StreamItem> {
    if data == "[DONE]" {
        return vec![StreamItem::Done];
    }
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    if let Some(error) = value.get("error") {
        return vec![StreamItem::Error(error_text(error))];
    }
    match format {
        Format::OpenAi => {
            let Some(choice) = value.get("choices").and_then(|c| c.get(0)) else {
                return Vec::new();
            };
            let mut items = Vec::new();
            if let Some(text) = choice
                .pointer("/delta/content")
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
            {
                items.push(StreamItem::Text(text.to_string()));
            }
            match choice.get("finish_reason").and_then(Value::as_str) {
                Some("length") => items.push(StreamItem::Notice(
                    "The summary was cut off at the model’s output limit.".into(),
                )),
                Some("content_filter") => items.push(StreamItem::Notice(
                    "The provider’s content filter stopped the summary.".into(),
                )),
                _ => {}
            }
            items
        }
        Format::Anthropic => {
            let kind = event.or_else(|| value.get("type").and_then(Value::as_str));
            match kind {
                Some("content_block_delta") => value
                    .pointer("/delta/text")
                    .and_then(Value::as_str)
                    .filter(|_| value.pointer("/delta/type") == Some(&json!("text_delta")))
                    .map(|t| vec![StreamItem::Text(t.to_string())])
                    .unwrap_or_default(),
                Some("message_delta") => {
                    match value.pointer("/delta/stop_reason").and_then(Value::as_str) {
                        Some("max_tokens") => vec![StreamItem::Notice(
                            "The summary was cut off at the model’s output limit.".into(),
                        )],
                        Some("refusal") => vec![StreamItem::Notice(
                            "The model declined to summarise this document.".into(),
                        )],
                        _ => Vec::new(),
                    }
                }
                Some("message_stop") => vec![StreamItem::Done],
                _ => Vec::new(),
            }
        }
    }
}

fn error_text(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| error.as_str())
        .unwrap_or("The provider reported an error.")
        .to_string()
}

/// A readable message for a failed (non-2xx) response.
pub fn http_error(provider: &str, status: u16, body: &[u8]) -> String {
    let detail = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| {
            // `{error: {message}}`, `{error: "…"}`, `[{error: {message}}]` (Gemini), `{message}`.
            let v = v.get(0).cloned().unwrap_or(v);
            v.get("error")
                .map(error_text)
                .or_else(|| v.get("message").and_then(Value::as_str).map(str::to_string))
        })
        .unwrap_or_else(|| {
            String::from_utf8_lossy(body)
                .chars()
                .take(200)
                .collect::<String>()
                .trim()
                .to_string()
        });
    let hint = match status {
        401 | 403 => format!("{provider} didn’t accept the API key. Check it in Settings."),
        404 => format!("{provider} doesn’t know this model or address. Check Settings."),
        429 => format!("{provider} is rate limiting requests or the account is out of credit."),
        _ => format!("{provider} returned an error ({status})."),
    };
    if detail.is_empty() {
        hint
    } else {
        format!("{hint}\n{detail}")
    }
}

/// Splits a byte stream into server-sent events. Feed it chunks as they arrive.
#[derive(Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    event: Option<String>,
    data: Vec<String>,
}

pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

impl SseParser {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(end) = self.buffer.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches(['\n', '\r']);
            self.line(line, &mut events);
        }
        events
    }

    /// Flushes an event the stream ended without a blank line after.
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            let rest = String::from_utf8_lossy(&std::mem::take(&mut self.buffer)).into_owned();
            self.line(rest.trim_end_matches('\r'), &mut events);
        }
        self.line("", &mut events);
        events
    }

    fn line(&mut self, line: &str, events: &mut Vec<SseEvent>) {
        if line.is_empty() {
            if !self.data.is_empty() {
                events.push(SseEvent {
                    event: self.event.take(),
                    data: std::mem::take(&mut self.data).join("\n"),
                });
            }
            self.event = None;
        } else if line.starts_with(':') {
            // A comment, e.g. OpenRouter's keep-alive ": OPENROUTER PROCESSING".
        } else if let Some(value) = line.strip_prefix("data:") {
            self.data
                .push(value.strip_prefix(' ').unwrap_or(value).to_string());
        } else if let Some(value) = line.strip_prefix("event:") {
            self.event = Some(value.trim().to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(format: Format, key: Option<&str>) -> Endpoint {
        Endpoint {
            format,
            base_url: "https://example.test/v1/".into(),
            key: key.map(str::to_string),
            model: "some-model".into(),
            openrouter: false,
        }
    }

    #[test]
    fn every_provider_is_unique_and_complete() {
        for (i, p) in PROVIDERS.iter().enumerate() {
            assert!(PROVIDERS[..i].iter().all(|q| q.id != p.id), "{}", p.id);
            assert!(p.id == CUSTOM || p.base_url.starts_with("http"), "{}", p.id);
            assert_eq!(p.needs_key, p.key_url.is_some(), "{}", p.id);
        }
        assert_eq!(find("anthropic").unwrap().format, Format::Anthropic);
    }

    #[test]
    fn builds_openai_requests() {
        let req = endpoint(Format::OpenAi, Some("sk-1")).chat_request("sys", "hello");
        assert_eq!(req.url, "https://example.test/v1/chat/completions");
        assert!(
            req.headers
                .contains(&("Authorization", "Bearer sk-1".to_string()))
        );
        let body: Value = serde_json::from_slice(req.body.as_ref().unwrap()).unwrap();
        assert_eq!(body["model"], "some-model");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "hello");
        assert!(body.get("max_tokens").is_none());

        // Keyless local servers get no Authorization header.
        let req = endpoint(Format::OpenAi, None).chat_request("sys", "hello");
        assert!(req.headers.iter().all(|(name, _)| *name != "Authorization"));
    }

    #[test]
    fn builds_anthropic_requests() {
        let req = endpoint(Format::Anthropic, Some("sk-ant")).chat_request("sys", "hello");
        assert_eq!(req.url, "https://example.test/v1/messages");
        assert!(req.headers.contains(&("x-api-key", "sk-ant".to_string())));
        assert!(
            req.headers
                .contains(&("anthropic-version", ANTHROPIC_VERSION.to_string()))
        );
        let body: Value = serde_json::from_slice(req.body.as_ref().unwrap()).unwrap();
        assert_eq!(body["system"], "sys");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["max_tokens"], ANTHROPIC_MAX_TOKENS);
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn parses_model_lists() {
        let openrouter = br#"{"data":[
            {"id":"b/model","name":"B","context_length":200000,"pricing":{"prompt":"0.000003","completion":"0.000015"}},
            {"id":"a/model","context_length":8192,"pricing":{"prompt":"0","completion":"0"}}
        ]}"#;
        let models = parse_models(openrouter).unwrap();
        assert_eq!(models[0].id, "a/model");
        assert_eq!(models[1].context_tokens, Some(200_000));
        assert_eq!(models[1].prompt_price, Some(0.000003));
        assert_eq!(models[0].prompt_price, Some(0.0));

        let openai =
            br#"{"data":[{"id":"text-embedding-3-small"},{"id":"chat-1"},{"id":"tts-1"}]}"#;
        let ids: Vec<_> = parse_models(openai)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(ids, ["chat-1"]);

        let anthropic =
            br#"{"data":[{"id":"claude-x","display_name":"Claude X","max_input_tokens":1000000}]}"#;
        let models = parse_models(anthropic).unwrap();
        assert_eq!(models[0].name.as_deref(), Some("Claude X"));
        assert_eq!(models[0].context_tokens, Some(1_000_000));

        let gemini = br#"{"data":[{"id":"models/gemini-x"}]}"#;
        assert_eq!(parse_models(gemini).unwrap()[0].id, "gemini-x");

        assert!(parse_models(b"<html>").is_err());
    }

    #[test]
    fn splits_server_sent_events_across_chunks() {
        let mut parser = SseParser::default();
        assert!(parser.push(b"event: content_block_delta\r\nda").is_empty());
        let events = parser.push(b"ta: {\"a\":1}\r\n\r\n: keep-alive\n\ndata: [DONE]\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("content_block_delta"));
        assert_eq!(events[0].data, "{\"a\":1}");
        let rest = parser.finish();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].data, "[DONE]");
    }

    #[test]
    fn reads_openai_stream_events() {
        let delta = r#"{"choices":[{"delta":{"content":"Hi"},"finish_reason":null}]}"#;
        assert_eq!(
            parse_event(Format::OpenAi, None, delta),
            [StreamItem::Text("Hi".into())]
        );
        let reasoning = r#"{"choices":[{"delta":{"reasoning":"hmm","content":""}}]}"#;
        assert!(parse_event(Format::OpenAi, None, reasoning).is_empty());
        let cut = r#"{"choices":[{"delta":{},"finish_reason":"length"}]}"#;
        assert!(matches!(
            parse_event(Format::OpenAi, None, cut)[..],
            [StreamItem::Notice(_)]
        ));
        let error = r#"{"error":{"message":"Out of credits"}}"#;
        assert_eq!(
            parse_event(Format::OpenAi, None, error),
            [StreamItem::Error("Out of credits".into())]
        );
        assert_eq!(
            parse_event(Format::OpenAi, None, "[DONE]"),
            [StreamItem::Done]
        );
    }

    #[test]
    fn reads_anthropic_stream_events() {
        let text =
            r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Hi"}}"#;
        assert_eq!(
            parse_event(Format::Anthropic, Some("content_block_delta"), text),
            [StreamItem::Text("Hi".into())]
        );
        let thinking = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hmm"}}"#;
        assert!(parse_event(Format::Anthropic, Some("content_block_delta"), thinking).is_empty());
        let refusal = r#"{"type":"message_delta","delta":{"stop_reason":"refusal"}}"#;
        assert!(matches!(
            parse_event(Format::Anthropic, Some("message_delta"), refusal)[..],
            [StreamItem::Notice(_)]
        ));
        let error =
            r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
        assert_eq!(
            parse_event(Format::Anthropic, Some("error"), error),
            [StreamItem::Error("Overloaded".into())]
        );
        assert_eq!(
            parse_event(
                Format::Anthropic,
                Some("message_stop"),
                r#"{"type":"message_stop"}"#
            ),
            [StreamItem::Done]
        );
    }

    #[test]
    fn explains_http_errors() {
        let message = http_error(
            "OpenAI",
            401,
            br#"{"error":{"message":"Incorrect API key"}}"#,
        );
        assert!(message.starts_with("OpenAI didn’t accept the API key"));
        assert!(message.ends_with("Incorrect API key"));
        let gemini = http_error(
            "Google Gemini",
            400,
            br#"[{"error":{"code":400,"message":"Bad model"}}]"#,
        );
        assert!(gemini.ends_with("Bad model"));
        assert_eq!(http_error("X", 500, b""), "X returned an error (500).");
    }
}
