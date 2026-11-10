//! OpenAI-compatible assistant client.
//!
//! The provider key lives in `Settings` and is only ever sent in the
//! `Authorization` header of a request made from this module. Errors carry the
//! provider's status and a short response snippet — never the key, and nothing
//! is written to the log.

use crate::model::{CharacterKind, LocalizedText, NodeKind, Project};
use crate::settings::AiSettings;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub id: String,
    pub owned_by: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiReply {
    pub content: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<AiUsage>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(90))
        .timeout_write(Duration::from_secs(30))
        .build()
}

fn endpoint(base: &str, path: &str) -> String {
    format!("{}/{}", base.trim().trim_end_matches('/'), path)
}

fn auth(request: ureq::Request, key: &str) -> ureq::Request {
    if key.trim().is_empty() {
        request
    } else {
        request.set("Authorization", &format!("Bearer {}", key.trim()))
    }
}

fn describe_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            let snippet: String = body.chars().take(400).collect();
            let hint = match code {
                401 | 403 => " Проверьте API-ключ в настройках ИИ.",
                404 => " Проверьте URL провайдера: обычно он заканчивается на /v1.",
                429 => " Слишком много запросов или закончилась квота.",
                _ => "",
            };
            format!("Провайдер ответил {code}: {snippet}.{hint}")
        }
        ureq::Error::Transport(transport) => format!("Сеть недоступна: {transport}"),
    }
}

/// `GET {base}/models` — the provider's catalog, sorted by id.
pub fn list_models(settings: &AiSettings) -> Result<Vec<AiModelInfo>, String> {
    if settings.base_url.trim().is_empty() {
        return Err("Не задан URL провайдера (Настройки → ИИ).".into());
    }
    let request = auth(agent().get(&endpoint(&settings.base_url, "models")), &settings.api_key);
    let response = request.call().map_err(describe_error)?;
    let value: Value = response
        .into_json()
        .map_err(|err| format!("Провайдер вернул не JSON: {err}"))?;
    let items = value
        .get("data")
        .and_then(|data| data.as_array())
        .cloned()
        .unwrap_or_default();
    let mut models = Vec::new();
    for item in items {
        match item {
            Value::String(id) => models.push(AiModelInfo {
                id,
                owned_by: None,
            }),
            Value::Object(map) => {
                let Some(id) = map.get("id").and_then(|id| id.as_str()) else {
                    continue;
                };
                if id.trim().is_empty() {
                    continue;
                }
                models.push(AiModelInfo {
                    id: id.to_string(),
                    owned_by: map
                        .get("owned_by")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string()),
                });
            }
            _ => {}
        }
    }
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    if models.is_empty() {
        return Err("Провайдер не вернул ни одной модели.".into());
    }
    Ok(models)
}

enum PostError {
    Status(u16, String),
    Transport(String),
    Parse(String),
}

fn post_chat(url: &str, key: &str, body: &Value) -> Result<Value, PostError> {
    let response = auth(
        agent()
            .post(url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json"),
        key,
    )
    .send_json(body.clone())
    .map_err(|err| match err {
        ureq::Error::Status(code, response) => {
            let text = response.into_string().unwrap_or_default();
            PostError::Status(code, text.chars().take(400).collect())
        }
        ureq::Error::Transport(transport) => PostError::Transport(transport.to_string()),
    })?;
    response
        .into_json::<Value>()
        .map_err(|err| PostError::Parse(format!("Провайдер вернул не JSON: {err}")))
}

fn post_error_text(err: PostError) -> String {
    match err {
        PostError::Status(code, body) => {
            let hint = match code {
                401 | 403 => " Проверьте API-ключ в настройках ИИ.",
                404 => " Проверьте URL провайдера: обычно он заканчивается на /v1.",
                429 => " Слишком много запросов или закончилась квота.",
                _ => "",
            };
            format!("Провайдер ответил {code}: {body}.{hint}")
        }
        PostError::Transport(message) => format!("Сеть недоступна: {message}"),
        PostError::Parse(message) => message,
    }
}

/// `POST {base}/chat/completions` with a non-streaming reply.
pub fn chat(
    settings: &AiSettings,
    system: &str,
    messages: &[ChatMessage],
) -> Result<AiReply, String> {
    if settings.base_url.trim().is_empty() {
        return Err("Не задан URL провайдера (Настройки → ИИ).".into());
    }
    if settings.model.trim().is_empty() {
        return Err("Не выбрана модель (Настройки → ИИ).".into());
    }
    let mut payload = Vec::new();
    if !system.trim().is_empty() {
        payload.push(json!({ "role": "system", "content": system }));
    }
    for message in messages {
        let role = match message.role.as_str() {
            "assistant" | "system" => message.role.as_str(),
            _ => "user",
        };
        payload.push(json!({ "role": role, "content": message.content }));
    }
    let url = endpoint(&settings.base_url, "chat/completions");
    let full = json!({
        "model": settings.model.trim(),
        "messages": payload,
        "temperature": 0.4,
        "max_tokens": 1600,
        "stream": false
    });
    let value = match post_chat(&url, &settings.api_key, &full) {
        Ok(value) => value,
        // Reasoning-only models reject sampling parameters: retry with the
        // smallest body every compatible provider accepts.
        Err(PostError::Status(400, _)) => {
            let minimal = json!({
                "model": settings.model.trim(),
                "messages": payload,
                "stream": false
            });
            post_chat(&url, &settings.api_key, &minimal).map_err(post_error_text)?
        }
        Err(err) => return Err(post_error_text(err)),
    };

    let choice = value
        .get("choices")
        .and_then(|choices| choices.as_array())
        .and_then(|choices| choices.first())
        .cloned()
        .unwrap_or(Value::Null);
    let content = match choice.get("message").and_then(|message| message.get("content")) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    if content.trim().is_empty() {
        let reason = choice
            .get("finish_reason")
            .and_then(|reason| reason.as_str())
            .unwrap_or("—");
        return Err(format!(
            "Провайдер вернул пустой ответ (finish_reason: {reason})."
        ));
    }
    let usage = value.get("usage").map(|usage| AiUsage {
        prompt_tokens: usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        completion_tokens: usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
    });
    Ok(AiReply {
        content,
        model: value
            .get("model")
            .and_then(|model| model.as_str())
            .unwrap_or(settings.model.trim())
            .to_string(),
        finish_reason: choice
            .get("finish_reason")
            .and_then(|reason| reason.as_str())
            .map(|reason| reason.to_string()),
        usage,
    })
}

// ---------------------------------------------------------------------------
// Project context for the system prompt
// ---------------------------------------------------------------------------

fn text_preview(text: &LocalizedText, locale: &str) -> String {
    let raw = text
        .get(locale)
        .or_else(|| text.values().next())
        .map(|value| value.as_str())
        .unwrap_or("");
    let single = raw.replace('\n', " ");
    if single.chars().count() > 90 {
        let cut: String = single.chars().take(90).collect();
        format!("{cut}…")
    } else {
        single
    }
}

fn params_preview(params: &Value, locale: &str) -> String {
    if let Some(object) = params.as_object() {
        let mut parts: Vec<String> = Vec::new();
        for (key, value) in object {
            let rendered = match value {
                Value::Object(map) if map.contains_key(locale) || !map.is_empty() => {
                    let localized: LocalizedText =
                        serde_json::from_value(value.clone()).unwrap_or_default();
                    format!("\"{}\"", text_preview(&localized, locale))
                }
                Value::String(text) => format!("\"{}\"", text.replace('\n', " ")),
                Value::Null => continue,
                other => other.to_string(),
            };
            parts.push(format!("{key}={rendered}"));
        }
        let joined = parts.join(", ");
        return joined.chars().take(180).collect();
    }
    String::new()
}

/// Compact, token-cheap description of the project for the assistant.
pub fn project_context(project: &Project, scene_id: Option<&str>) -> String {
    let locale = if project.meta.default_locale.is_empty() {
        "ru"
    } else {
        project.meta.default_locale.as_str()
    };
    let mut out = String::new();
    out.push_str(&format!(
        "Проект: «{}». Локали: {}. Стартовая сцена: {}.\n",
        project.meta.title,
        project.locales.join(", "),
        project
            .meta
            .start_scene
            .as_deref()
            .and_then(|id| project.scene_by_id(id))
            .map(|scene| format!("{} ({})", scene.title, scene.id))
            .unwrap_or_else(|| "не задана".into())
    ));

    if !project.characters.is_empty() {
        let chars = project
            .characters
            .iter()
            .map(|character| {
                let kind = match character.kind {
                    CharacterKind::Narrator => "рассказчик",
                    CharacterKind::Thought => "мысли",
                    CharacterKind::Character => "персонаж",
                };
                format!("{} ({}, id {})", character.name, kind, character.id)
            })
            .collect::<Vec<_>>()
            .join("; ");
        out.push_str(&format!("Персонажи: {chars}.\n"));
    }
    if !project.variables.is_empty() {
        let vars = project
            .variables
            .iter()
            .map(|variable| format!("{} ({:?})", variable.name, variable.var_type))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("Переменные: {vars}.\n"));
    }

    out.push_str("\nСцены (id | название | метка | блоков):\n");
    let labels = crate::generator::scene_labels(project);
    for scene in project.scenes.iter().take(40) {
        let label = labels.get(&scene.id).cloned().unwrap_or_default();
        out.push_str(&format!(
            "- {} | {} | {} | {}\n",
            scene.id,
            scene.title,
            label,
            scene.nodes.len()
        ));
    }

    if let Some(scene) = scene_id
        .and_then(|id| project.scene_by_id(id))
        .or_else(|| project.scenes.first())
    {
        out.push_str(&format!(
            "\nТекущая сцена: {} ({}), блоки по порядку:\n",
            scene.title, scene.id
        ));
        for (index, node) in scene.nodes.iter().take(60).enumerate() {
            out.push_str(&format!(
                "{}. {} [{}] {}\n",
                index + 1,
                node.kind,
                node.id,
                params_preview(&node.params, locale)
            ));
        }
    }
    out
}

/// Block catalog shown to the model so proposed `add_block` actions are valid.
pub fn block_catalog() -> &'static str {
    "say: who (id персонажа), text {ru: \"…\"}, image, cps\n\
narrate: text {ru: \"…\"}\n\
scene: bg (путь images/…), transition (dissolve/fade/…)\n\
show: who, image (поза), asset, position (left/center/right)\n\
hide: who\n\
pose: who, image, asset\n\
anim: who, preset (shake/bounce/flash/slide_left/slide_right/zoom)\n\
menu: prompt {ru: \"…\"}, options [{id, text {ru: \"…\"}, target (scene id), condition}]\n\
jump: target (scene id)\n\
if: mode (builder|expression), variable, op (eq/ne/gt/gte/lt/lte/is_true/is_false), value, thenTarget, elseTarget, hasElse\n\
set: variable, op (set/add/sub/mul), value\n\
music: action (play/stop/queue), file, loop, fadeout\n\
sound: file, loop\n\
pause: seconds, hard\n\
pointclick: bg, prompt {ru: \"…\"}, hotspots [{id, label {ru: \"…\"}, shape (rect|polygon), x,y,w,h (0..1), points, action (jump|tooltip|return), target}]\n\
code: code, language (renpy|python)"
}

pub fn system_prompt(project: &Project, scene_id: Option<&str>) -> String {
    format!(
        "Ты — встроенный ассистент RenPy Studio, редактора визуальных новелл на Ren'Py.\n\
Помогаешь автору с сюжетом, структурой сцен, диалогами и настройкой блоков. Отвечай коротко и по делу, на языке пользователя (по умолчанию русский). Опирайся только на контекст проекта ниже и не выдумывай id.\n\n\
Доступные блоки:\n{catalog}\n\n\
Если пользователь просит что-то изменить, в самом конце ответа добавь ровно один блок кода:\n\
```json\n{{\"actions\": [{{\"type\": \"add_block\", \"sceneId\": \"…\", \"kind\": \"say\", \"params\": {{\"who\": \"…\", \"text\": {{\"ru\": \"…\"}}}}}}]}}\n```\n\
Допустимые действия: add_scene {{title}}, add_block {{sceneId, kind, params}}, set_start_scene {{sceneId}}.\n\
Правила: только существующие sceneId из контекста; kind — из списка блоков; params — валидный объект для блока; не больше 5 действий. Если изменения не нужны — блок не добавляй.\n\n\
Контекст проекта:\n{context}",
        catalog = block_catalog(),
        context = project_context(project, scene_id)
    )
}

/// Kinds the assistant may add. Keeps a hallucinated `kind` from reaching the store.
#[allow(dead_code)]
pub fn is_known_kind(kind: &str) -> bool {
    NodeKind::ALL.contains(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_lists_scenes_and_blocks() {
        let mut project = Project::default();
        project.meta.title = "Тест".into();
        project.meta.default_locale = "ru".into();
        let mut scene = crate::model::Scene {
            id: "sc_a".into(),
            title: "Пролог".into(),
            slug: "prologue".into(),
            label_name: None,
            summary: String::new(),
            color: None,
            nodes: Vec::new(),
        };
        scene.nodes.push(crate::model::Node::new(
            NodeKind::SAY,
            json!({ "who": null, "text": { "ru": "Привет" } }),
        ));
        project.scenes.push(scene);
        project.meta.start_scene = Some("sc_a".into());
        let context = project_context(&project, Some("sc_a"));
        assert!(context.contains("Пролог"));
        assert!(context.contains("say"));
        assert!(context.contains("Привет"));
        assert!(context.contains("scene_prologue"));
    }

    #[test]
    fn defaults_do_not_touch_the_network() {
        // An empty provider URL must fail before any request is attempted.
        let settings = AiSettings {
            base_url: String::new(),
            api_key: "sk-test".into(),
            model: "test".into(),
        };
        assert!(list_models(&settings).is_err());
        assert!(chat(&settings, "", &[]).is_err());
    }

    #[test]
    fn catalog_covers_pointclick() {
        assert!(block_catalog().contains("pointclick"));
        assert!(is_known_kind(NodeKind::POINTCLICK));
        assert!(!is_known_kind("nonsense"));
    }
}
