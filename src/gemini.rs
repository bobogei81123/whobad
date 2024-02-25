use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<Content>,
    safety_settings: Vec<SafetySetting>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Part {
    text: String,
}

impl Content {
    fn new(text: String) -> Self {
        Self {
            parts: vec![Part { text }],
        }
    }
}

#[derive(Serialize)]
struct SafetySetting {
    category: HarmCategory,
    threshold: HarmBlockThreshold,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::enum_variant_names)]
enum HarmCategory {
    HarmCategoryHarassment,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
enum HarmBlockThreshold {
    BlockLowAndAbove,
    BlockMediumAndAbove,
    BlockOnlyHigh,
    BlockNone,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Content,
    #[allow(dead_code)]
    finish_reason: FinishReason,
}

impl Candidate {
    fn into_text(self) -> String {
        self.content
            .parts
            .into_iter()
            .map(|p| p.text)
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)]
enum FinishReason {
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Other,
}

const GEMINI_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent";

/// Returns the response from Gemini given a prompt.
pub async fn ask_gemini(prompt: String) -> anyhow::Result<String> {
    tracing::info!("Asking Gemini:\n{}", prompt);

    let query = GenerateContentRequest {
        contents: vec![Content::new(prompt)],
        safety_settings: vec![SafetySetting {
            category: HarmCategory::HarmCategoryHarassment,
            threshold: HarmBlockThreshold::BlockOnlyHigh,
        }],
    };
    let result = reqwest::Client::new()
        .post(GEMINI_URL)
        .query(&[("key", &Config::get().gemini_apikey)])
        .json(&query)
        .send()
        .await
        .context("Gemini request failed")?
        .json::<GenerateContentResponse>()
        .await
        .context("Cannot parse Gemini response into JSON")?;

    if result.candidates.is_empty() {
        bail!("Gemini returned no candidates");
    }

    // move out candidates[0]
    Ok(result.candidates.into_iter().next().unwrap().into_text())
}
