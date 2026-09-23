use serde_json::Value;
use std::sync::OnceLock;

fn dictionary(domain: &str) -> &'static Value {
    static WORK: OnceLock<Value> = OnceLock::new();
    static GRAPH: OnceLock<Value> = OnceLock::new();
    if domain == "graph" {
        GRAPH.get_or_init(|| serde_json::from_str(include_str!("../resources/graph.json")).unwrap())
    } else {
        WORK.get_or_init(|| {
            serde_json::from_str(include_str!("../resources/workbench.json")).unwrap()
        })
    }
}
pub fn tr(language: &str, key: &str) -> &'static str {
    let (domain, key) = key.split_once('.').unwrap_or(("work", key));
    dictionary(domain)[language][key].as_str().unwrap_or("")
}
pub fn help(language: &str) -> Vec<&'static str> {
    dictionary("graph")[language]["helpItems"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect()
}
pub fn diagnostic(message: &str, language: &str) -> String {
    static DIAGNOSTICS: OnceLock<Value> = OnceLock::new();
    static PREFIXES: OnceLock<Value> = OnceLock::new();
    let dictionary = DIAGNOSTICS.get_or_init(|| {
        serde_json::from_str(include_str!("../resources/diagnostics.json")).unwrap()
    });
    if let Some(translated) = dictionary[message][language].as_str() {
        return translated.into();
    }
    if language != "zh" {
        if let Some(count) = message
            .strip_prefix("另有 ")
            .and_then(|s| s.strip_suffix(" 条同类警告未显示。"))
        {
            return if language == "en" {
                format!("{count} additional warnings of this kind were omitted.")
            } else {
                format!("同種の警告をさらに {count} 件省略しました。")
            };
        }
        if let Some(path) = message.strip_suffix(" 包含解析错误，依赖结果可能不完整。")
        {
            return if language == "en" {
                format!("{path} contains parse errors; dependency results may be incomplete.")
            } else {
                format!(
                    "{path} に構文解析エラーがあります。依存関係の結果が不完全な可能性があります。"
                )
            };
        }
        if let Some(count) = message
            .strip_prefix("目录项超过 ")
            .and_then(|s| s.strip_suffix("，扫描提前结束；当前结果仅代表已扫描部分。"))
        {
            return if language == "en" {
                format!("Directory entries exceeded {count}. The scan stopped early; results cover only the scanned portion.")
            } else {
                format!("ディレクトリエントリが {count} 件を超えたため、スキャンを途中で終了しました。結果はスキャン済みの部分のみを対象とします。")
            };
        }
    }
    if let Some(parser) = message.strip_suffix(" parser unavailable") {
        return match language {
            "zh" => format!("{parser} 解析器不可用。"),
            "ja" => format!("{parser} の解析器を利用できません。"),
            _ => message.into(),
        };
    }
    if let Some((path, reason)) = message
        .strip_prefix("无法解析 ")
        .and_then(|s| s.rsplit_once('：'))
    {
        let reason = diagnostic(reason, language);
        return match language {
            "en" => format!("Could not parse {path}: {reason}"),
            "ja" => format!("{path} の構文解析に失敗しました: {reason}"),
            _ => format!("无法解析 {path}：{reason}"),
        };
    }
    let prefixes = PREFIXES
        .get_or_init(|| serde_json::from_str(include_str!("../resources/prefixes.json")).unwrap());
    for pair in prefixes.as_array().unwrap() {
        if let Some(suffix) = message.strip_prefix(pair[0].as_str().unwrap()) {
            if let Some(prefix) = pair[1][language].as_str() {
                return format!("{prefix}{suffix}");
            }
        }
    }
    message.into()
}
pub fn short(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.into()
    } else {
        format!(
            "{}…",
            text.chars()
                .take(limit.saturating_sub(1))
                .collect::<String>()
        )
    }
}
