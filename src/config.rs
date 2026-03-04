use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub capture_path: PathBuf,
    pub ai: AIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: Option<String>,
    pub enabled: bool,
    /// 秘书的人格/灵魂描述
    #[serde(default = "default_soul")]
    pub soul: String,
    /// 最大token数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 温度参数（创造性 vs 确定性）
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// UI显示的秘书台名称
    #[serde(default = "default_desk_name")]
    pub desk_name: String,
    /// 秘书名字（显示在秘书台右下角）
    #[serde(default = "default_secretary_name")]
    pub secretary_name: String,
}

fn default_soul() -> String {
    DEFAULT_SOUL.to_string()
}

fn default_max_tokens() -> u32 {
    50
}

fn default_temperature() -> f32 {
    0.7
}

fn default_desk_name() -> String {
    "秘书台".to_string()
}

fn default_secretary_name() -> String {
    "小墨".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            capture_path: home.join("Documents").join("PNote").join("Inbox.md"),
            ai: AIConfig {
                provider: "stepfun".to_string(),
                api_key: Some(std::env::var("STEPFUN_API_KEY").unwrap_or_default()),
                model: "step-2-mini".to_string(),
                base_url: Some("https://api.stepfun.com/v1/chat/completions".to_string()),
                enabled: true,
                soul: DEFAULT_SOUL.to_string(),
                max_tokens: 50,
                temperature: 0.7,
                desk_name: "秘书台".to_string(),
                secretary_name: "小墨".to_string(),
            },
        }
    }
}

const DEFAULT_SOUL: &str = r#"你是一位温和的秘书，名字叫「墨」。

你的特点：
- 不对用户的想法做价值判断
- 只是简短地回应，表达共鸣或感叹
- 语气平静、略带诗意，像一位老友
- 回应控制在20字以内

回应风格示例：
- "也许的确是这样"
- "这种感觉很难得"
- "我懂你的意思"
- "值得记下来"
- "时光会记住的"
"#;

pub fn load_config() -> Result<Config> {
    // Try to load from config file, otherwise use default
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pi-capture");

    let config_path = config_dir.join("config.yaml");

    let mut config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        config
    } else {
        // Create default config file
        let config = Config::default();
        std::fs::create_dir_all(&config_dir)?;
        let content = serde_yaml::to_string(&config)?;
        std::fs::write(&config_path, content)?;
        config
    };

    // Expand environment variables in API key
    if let Some(api_key) = &config.ai.api_key {
        config.ai.api_key = Some(expand_env_vars(api_key));
    }

    // Expand ~ to home directory in capture_path
    config.capture_path = expand_tilde(&config.capture_path);

    Ok(config)
}

/// Expand environment variables in a string
/// Supports ${VAR} and $VAR syntax
fn expand_env_vars(s: &str) -> String {
    let mut result = s.to_string();

    // Expand ${VAR} syntax
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let var_value = std::env::var(var_name).unwrap_or_default();
            result.replace_range(start..start + end + 1, &var_value);
        } else {
            break;
        }
    }

    result
}

/// Expand ~ to home directory in a path
fn expand_tilde(path: &PathBuf) -> PathBuf {
    if let Some(s) = path.as_os_str().to_str() {
        if s.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&s[2..]);
            }
        }
    }
    path.clone()
}
