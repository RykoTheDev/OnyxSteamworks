use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::Commit;

fn build_prompt(commits: &[Commit]) -> String {
    let list = commits
        .iter()
        .map(|c| format!("- {}", c.subject))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are writing player-facing patch notes for a video game update.\n\
         Here are the commit messages from this build:\n{list}\n\n\
         Write short, friendly patch notes grouped into clear sections using Markdown '## ' \
         headings and '- ' bullets. Only describe changes that appear in the commits above; \
         never invent features that are not listed. Keep each line concise."
    )
}

pub fn generate_llm(
    base_url: &str,
    model: &str,
    api_key: &str,
    commits: &[Commit],
    cancel: &AtomicBool,
) -> Result<String, String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": build_prompt(commits) }],
        "temperature": 0.4,
        "stream": false,
    });
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(90))
        .build();
    let mut request = agent.post(&url).set("Content-Type", "application/json");
    if !api_key.trim().is_empty() {
        request = request.set("Authorization", &format!("Bearer {}", api_key.trim()));
    }
    let response = request.send_json(body).map_err(|e| e.to_string())?;
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }
    let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "the endpoint returned no content".to_string())
}
