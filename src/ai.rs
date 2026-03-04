use crate::app::Entry;
use crate::config::AIConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct AISecretary {
    config: AIConfig,
    sender: mpsc::UnboundedSender<String>,
    client: reqwest::Client,
}

impl AISecretary {
    pub fn new(config: AIConfig, sender: mpsc::UnboundedSender<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            config,
            sender,
            client,
        }
    }

    pub fn request_analysis(&self, text: String, _history: Vec<Entry>) -> Result<(), ()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Skip API call for very short input
        if text.len() < 3 {
            return Ok(());
        }

        let sender = self.sender.clone();
        let config = self.config.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let response = if config.provider == "mock" {
                // Mock mode for testing
                tokio::time::sleep(Duration::from_millis(300)).await;
                Ok(generate_mock_response(&text))
            } else {
                // OpenAI-compatible API
                call_api(&client, &config, &text).await
            };

            match response {
                Ok(text) => {
                    let _ = sender.send(text);
                }
                Err(e) => {
                    let _ = sender.send(format!("💤 秘书暂时离线: {}", e));
                }
            }
        });

        Ok(())
    }
}

// OpenAI-compatible API (works with DeepSeek, Moonshot, Zhipu, Groq, etc.)
async fn call_api(
    client: &reqwest::Client,
    config: &AIConfig,
    text: &str,
) -> Result<String> {
    let api_key = config
        .api_key
        .as_ref()
        .ok_or_else(|| anyhow!("API key not set"))?;

    // Use configured base_url or default to DeepSeek
    let url = config
        .base_url
        .as_deref()
        .unwrap_or("https://api.deepseek.com/v1/chat/completions");

    let request = ChatRequest {
        model: config.model.clone(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: config.soul.clone(),
            },
            Message {
                role: "user".to_string(),
                content: format!("用户刚记录的想法：{}\n\n请简短回应（20字以内），表达共鸣或感叹即可。", text),
            },
        ],
        max_tokens: Some(config.max_tokens),
        temperature: Some(config.temperature),
    };

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("API error: {}", error_text));
    }

    let result: ChatResponse = response.json().await?;

    Ok(result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "💭".to_string()))
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

// Mock response for testing - generates response based on input content
fn generate_mock_response(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();

    // 根据输入特征生成回应
    let len = text.len();

    // 提取一些特征
    let has_question = text.contains('?') || text.contains('？');
    let has_exclaim = text.contains('!') || text.contains('！');

    // 基于特征生成共鸣式回应（20字以内）
    let response = if has_question {
        let templates = vec![
            "这个问题值得想想。",
            "追问本身就是答案。",
            "疑问中藏着光。",
            "你提出了一个好问题。",
        ];
        templates[(hash as usize) % templates.len()]
    } else if has_exclaim {
        let templates = vec![
            "能感受到这份心情。",
            "强烈的感受值得记录。",
            "这份热情很珍贵。",
            "情绪流动起来了。",
        ];
        templates[(hash as usize) % templates.len()]
    } else if len < 10 {
        let templates = vec![
            "简短却有力量。",
            "言简意赅。",
            "有时候一字千金。",
            "留白也是表达。",
        ];
        templates[(hash as usize) % templates.len()]
    } else if len > 50 {
        let templates = vec![
            "思绪很长，慢慢说。",
            "长文载着深想。",
            "娓娓道来，很好。",
            " detailed 的思考。",
        ];
        templates[(hash as usize) % templates.len()]
    } else {
        // 通用共鸣回应
        let templates = vec![
            "也许的确是这样。",
            "这种感觉很难得。",
            "我懂你的意思。",
            "值得记下来。",
            "时光会记住的。",
            "这样的想法很特别。",
            "有时候就是这样。",
            "嗯，是这样。",
            "记下这个想法真好。",
            "我也有过类似的感受。",
            "平淡中见真意。",
            "这一刻的想法很真实。",
        ];
        templates[(hash as usize) % templates.len()]
    };

    response.to_string()
}
