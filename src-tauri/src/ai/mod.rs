//! AI summaries (TL;DR). The page builds the prompts and shows the result;
//! this side picks the provider from settings, adds the API key and streams
//! the reply back, so keys never reach the web view.

mod http;
mod keychain;
pub mod providers;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tokio::sync::Notify;

use crate::settings;
use providers::{CUSTOM, Endpoint, Format, ModelInfo, Provider, SseParser, StreamItem};

/// What the user configured for one provider (in settings.json).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderSettings {
    pub model: String,
    /// Only used by local and custom providers.
    pub base_url: Option<String>,
    /// Only used by the custom provider.
    pub format: Option<Format>,
    /// From the provider's model list, or typed by the user.
    pub context_tokens: Option<u64>,
    /// US dollars per token, from the model list (OpenRouter).
    pub prompt_price: Option<f64>,
    pub completion_price: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub provider: String,
    pub providers: BTreeMap<String, ProviderSettings>,
    /// The user's own summary instructions; `None` uses the built-in ones.
    pub instructions: Option<String>,
    /// Cloud providers the user agreed to send documents to.
    pub allowed: Vec<String>,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: "openrouter".into(),
            providers: BTreeMap::new(),
            instructions: None,
            allowed: Vec::new(),
        }
    }
}

/// Requests that can still be cancelled, by the page's request id.
#[derive(Default)]
pub struct AiState(Mutex<HashMap<u32, Arc<Notify>>>);

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StreamMessage {
    Text { text: String },
    Notice { text: String },
}

fn provider(id: &str) -> Result<&'static Provider, String> {
    providers::find(id).ok_or_else(|| format!("Unknown AI provider “{id}”."))
}

fn base_url(p: &Provider, config: &ProviderSettings) -> String {
    let custom = config
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|url| p.editable_url && !url.is_empty());
    custom.unwrap_or(p.base_url).to_string()
}

/// The keychain entry for a provider's key. A custom provider's key belongs to
/// its address, so changing the address never sends the key somewhere new.
fn key_account(app: &AppHandle, p: &Provider) -> String {
    if p.id == CUSTOM {
        let config = settings::current(app).ai.providers.get(CUSTOM).cloned();
        format!("{CUSTOM} {}", base_url(p, &config.unwrap_or_default()))
    } else {
        p.id.to_string()
    }
}

/// Resolves provider settings and the key. Blocking (keychain).
fn endpoint(
    app: &AppHandle,
    id: &str,
    need_model: bool,
) -> Result<(Endpoint, &'static Provider), String> {
    let p = provider(id)?;
    let config = settings::current(app)
        .ai
        .providers
        .get(id)
        .cloned()
        .unwrap_or_default();
    let base_url = base_url(p, &config);
    if base_url.is_empty() {
        return Err(format!(
            "Enter the {} provider’s address in Settings.",
            p.name
        ));
    }
    let model = Some(config.model.trim())
        .filter(|m| !m.is_empty())
        .or(p.default_model)
        .unwrap_or_default()
        .to_string();
    if need_model && model.is_empty() {
        return Err(format!("Choose a model for {} in Settings.", p.name));
    }
    let key = keychain::get(&key_account(app, p))?.filter(|k| !k.is_empty());
    if p.needs_key && key.is_none() {
        return Err(format!("Add your {} API key in Settings.", p.name));
    }
    let format = if p.id == CUSTOM {
        config.format.unwrap_or_default()
    } else {
        p.format
    };
    let endpoint = Endpoint {
        format,
        base_url,
        key,
        model,
        openrouter: p.id == "openrouter",
    };
    Ok((endpoint, p))
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

fn network_error(p: &Provider, endpoint: &Endpoint, error: String) -> String {
    if p.local {
        format!(
            "Couldn’t reach {} at {}. Is it running?\n{error}",
            p.name, endpoint.base_url
        )
    } else {
        format!("Couldn’t reach {}: {error}", p.name)
    }
}

#[tauri::command]
pub fn ai_providers() -> &'static [Provider] {
    providers::PROVIDERS
}

/// Whether a key is saved for the provider. The key itself never leaves Rust.
#[tauri::command]
pub async fn ai_key_status(app: AppHandle, provider: String) -> Result<bool, String> {
    let p = self::provider(&provider)?;
    blocking(move || Ok(keychain::get(&key_account(&app, p))?.is_some_and(|k| !k.is_empty()))).await
}

/// Saves the provider's key; an empty key removes it.
#[tauri::command]
pub async fn ai_set_key(app: AppHandle, provider: String, key: String) -> Result<(), String> {
    let p = self::provider(&provider)?;
    blocking(move || {
        let account = key_account(&app, p);
        match key.trim() {
            "" => keychain::delete(&account),
            key => keychain::set(&account, key),
        }
    })
    .await
}

/// The provider's models, for the picker in Settings.
#[tauri::command]
pub async fn ai_models(app: AppHandle, provider: String) -> Result<Vec<ModelInfo>, String> {
    let (endpoint, p) = blocking(move || endpoint(&app, &provider, false)).await?;
    let (status, body) = http::fetch(endpoint.models_request())
        .await
        .map_err(|e| network_error(p, &endpoint, e))?;
    if !(200..300).contains(&status) {
        return Err(providers::http_error(p.name, status, &body));
    }
    providers::parse_models(&body)
}

/// Sends one prompt to the selected provider and streams the reply as
/// `StreamMessage`s. Resolves when the reply is complete.
#[tauri::command]
pub async fn ai_complete(
    app: AppHandle,
    state: State<'_, AiState>,
    id: u32,
    system: String,
    prompt: String,
    on_message: Channel<StreamMessage>,
) -> Result<(), String> {
    let cancel = Arc::new(Notify::new());
    state.0.lock().unwrap().insert(id, cancel.clone());
    let result = tokio::select! {
        result = complete(app, system, prompt, on_message) => result,
        _ = cancel.notified() => Err("Cancelled.".into()),
    };
    state.0.lock().unwrap().remove(&id);
    result
}

async fn complete(
    app: AppHandle,
    system: String,
    prompt: String,
    on_message: Channel<StreamMessage>,
) -> Result<(), String> {
    let id = settings::current(&app).ai.provider;
    let (endpoint, p) = blocking(move || endpoint(&app, &id, true)).await?;
    let mut rx = http::send(endpoint.chat_request(&system, &prompt))?;

    let mut status = 0;
    let mut error_body = Vec::new();
    let mut parser = SseParser::default();
    let handle = |events: Vec<providers::SseEvent>| -> Result<(), String> {
        for event in events {
            for item in providers::parse_event(endpoint.format, event.event.as_deref(), &event.data)
            {
                let message = match item {
                    StreamItem::Text(text) => StreamMessage::Text { text },
                    StreamItem::Notice(text) => StreamMessage::Notice { text },
                    StreamItem::Error(error) => return Err(format!("{}: {error}", p.name)),
                    StreamItem::Done => continue,
                };
                on_message.send(message).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    };
    while let Some(event) = rx.recv().await {
        match event {
            http::Event::Status(code) => status = code,
            http::Event::Data(data) if (200..300).contains(&status) => handle(parser.push(&data))?,
            http::Event::Data(data) => error_body.extend(data),
            http::Event::Finished(result) => {
                result.map_err(|e| network_error(p, &endpoint, e))?;
                break;
            }
        }
    }
    if !(200..300).contains(&status) {
        return Err(providers::http_error(p.name, status, &error_body));
    }
    handle(parser.finish())
}

#[tauri::command]
pub fn ai_cancel(state: State<'_, AiState>, id: u32) {
    if let Some(cancel) = state.0.lock().unwrap().get(&id) {
        cancel.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_and_custom_addresses_can_change() {
        let config = ProviderSettings {
            base_url: Some(" http://elsewhere.test/v1 ".into()),
            ..Default::default()
        };
        let openai = providers::find("openai").unwrap();
        assert_eq!(base_url(openai, &config), "https://api.openai.com/v1");
        let ollama = providers::find("ollama").unwrap();
        assert_eq!(base_url(ollama, &config), "http://elsewhere.test/v1");
        assert_eq!(
            base_url(ollama, &ProviderSettings::default()),
            "http://localhost:11434/v1"
        );
    }

    #[test]
    fn settings_without_ai_section_load() {
        let s: AiSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.provider, "openrouter");
        let s: AiSettings = serde_json::from_str(
            r#"{"provider":"custom","providers":{"custom":{"format":"anthropic","model":"m"}}}"#,
        )
        .unwrap();
        assert_eq!(s.providers["custom"].format, Some(Format::Anthropic));
    }
}
