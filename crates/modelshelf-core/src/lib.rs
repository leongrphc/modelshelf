mod database;
mod domain;
pub mod storage;
use anyhow::{bail, Result};
pub use database::Database;
pub use domain::*;

pub fn parse_repository(input: &str) -> Result<String> {
    let input = input.trim();
    let id = if input.starts_with("https://") {
        let url = url::Url::parse(input)?;
        if url.host_str() != Some("huggingface.co")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            bail!("Only official Hugging Face model URLs are supported");
        }
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        if parts.len() != 2
            && !(parts.len() >= 4 && matches!(parts[2], "tree" | "blob" | "resolve"))
        {
            bail!("Expected a Hugging Face model repository URL");
        }
        format!("{}/{}", parts[0], parts[1])
    } else {
        input.to_owned()
    };
    let parts: Vec<_> = id.split('/').collect();
    if parts.len() != 2
        || matches!(parts[0], "datasets" | "spaces")
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
    {
        bail!("Expected owner/model with letters, numbers, '.', '-' or '_'");
    }
    Ok(id)
}

pub fn quantization(name: &str) -> Option<String> {
    let upper = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_uppercase();
    for (index, _) in upper.char_indices() {
        if index > 0 && !matches!(upper.as_bytes()[index - 1], b'-' | b'_' | b'.') {
            continue;
        }
        let part = upper[index..]
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        let numeric = part
            .strip_prefix("IQ")
            .or_else(|| part.strip_prefix('Q'))
            .and_then(|s| s.bytes().next())
            .is_some_and(|b| b.is_ascii_digit());
        if matches!(part, "F16" | "F32" | "BF16") || numeric {
            return Some(part.into());
        }
    }
    None
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_parsing() {
        assert_eq!(
            parse_repository("https://huggingface.co/owner/model/tree/main").unwrap(),
            "owner/model"
        );
        for value in [
            "https://evil.test/a/b",
            "../a",
            "a/b/c",
            "https://huggingface.co@evil.test/a/b",
            "datasets/foo",
        ] {
            assert!(parse_repository(value).is_err());
        }
    }
    #[test]
    fn quantizations() {
        assert_eq!(
            quantization("Qwen3-8B_Q4_K_M.gguf").as_deref(),
            Some("Q4_K_M")
        );
        assert_eq!(quantization("Qwen3.gguf"), None);
        assert_eq!(quantization("model-Q4_K_M.gguf").as_deref(), Some("Q4_K_M"));
        assert_eq!(
            quantization("model-IQ3_XXS.gguf").as_deref(),
            Some("IQ3_XXS")
        );
        assert_eq!(quantization("model.safetensors"), None);
    }
}
