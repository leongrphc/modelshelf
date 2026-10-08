use anyhow::{bail, Context, Result};
use modelshelf_core::{RemoteFile, Repository};
use reqwest::{Client, Response, Url};
use serde_json::Value;
use std::time::Duration;

pub struct Hub {
    client: Client,
    token: Option<String>,
}

/// A provider supplies immutable metadata; it never writes into model storage.
pub trait ModelProvider {
    fn search_models(
        &self,
        query: &str,
        sort: &str,
        task: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Repository>>> + Send;
    fn model_details(
        &self,
        repo: &str,
        revision: &str,
    ) -> impl std::future::Future<Output = Result<Repository>> + Send;
}
impl ModelProvider for Hub {
    async fn search_models(&self, query: &str, sort: &str, task: &str) -> Result<Vec<Repository>> {
        self.search(query, sort, task).await
    }
    async fn model_details(&self, repo: &str, revision: &str) -> Result<Repository> {
        self.details(repo, revision).await
    }
}

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent("ModelShelf/0.1")
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(45))
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}

fn allowed(url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && (host == "huggingface.co"
            || host.ends_with(".huggingface.co")
            || host.ends_with(".hf.co")
            || host.ends_with(".xethub.hf.co"))
}

/// Follow only provider-controlled HTTPS origins; bearer credentials never leave the API host.
pub async fn get(
    client: &Client,
    mut url: Url,
    token: Option<&str>,
    range: Option<(u64, &str)>,
) -> Result<Response> {
    for _ in 0..10 {
        if !allowed(&url) {
            bail!("Provider redirected to an unsupported host");
        }
        let mut request = client.get(url.clone());
        if url.host_str() == Some("huggingface.co") {
            if let Some(token) = token {
                request = request.bearer_auth(token);
            }
        }
        if let Some((offset, etag)) = range {
            request = request
                .header("Range", format!("bytes={offset}-"))
                .header("If-Range", etag);
        }
        let response = request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Network request failed or timed out"))?;
        if response.status().is_redirection() {
            let next = response
                .headers()
                .get("location")
                .context("Redirect missing location")?
                .to_str()?;
            url = url.join(next)?;
        } else {
            return Ok(response);
        }
    }
    bail!("Too many provider redirects")
}

pub fn response_error(response: &Response) -> Result<()> {
    match response.status().as_u16() {
        200..=299 => Ok(()),
        401 => bail!("HTTP 401: connect a valid Hugging Face token"),
        403 => bail!("HTTP 403: access denied; obtain gated-model approval on Hugging Face"),
        404 => bail!("HTTP 404: repository, revision, or file not found"),
        429 => bail!("HTTP 429: provider rate limit; retry later"),
        code => bail!("Provider HTTP {code}"),
    }
}

pub fn file_url(repo: &str, revision: &str, path: &str) -> Result<Url> {
    let repo = modelshelf_core::parse_repository(repo)?;
    modelshelf_core::storage::validate_relative(path)?;
    let mut url = Url::parse("https://huggingface.co")?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Invalid origin"))?
        .extend(repo.split('/'))
        .push("resolve")
        .push(revision)
        .extend(path.split('/'));
    Ok(url)
}

fn repository(v: Value) -> Repository {
    let string = |key: &str| v[key].as_str().map(str::to_owned);
    Repository {
        id: string("id").unwrap_or_default(),
        sha: string("sha").unwrap_or_default(),
        task: string("pipeline_tag"),
        downloads: v["downloads"].as_u64().unwrap_or(0),
        likes: v["likes"].as_u64().unwrap_or(0),
        tags: v["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        license: v["cardData"]["license"].as_str().map(str::to_owned),
        updated_at: string("lastModified"),
        gated: v["gated"]
            .as_bool()
            .unwrap_or_else(|| v["gated"].as_str().is_some_and(|s| s != "false")),
        files: vec![],
        readme: String::new(),
        revisions: vec![],
    }
}

impl Hub {
    pub fn new(token: Option<String>) -> Result<Self> {
        Ok(Self {
            client: client()?,
            token,
        })
    }
    async fn fetch(&self, url: Url) -> Result<Response> {
        let response = get(&self.client, url, self.token.as_deref(), None).await?;
        response_error(&response)?;
        Ok(response)
    }
    pub async fn username(&self) -> Result<String> {
        let v: Value = self
            .fetch(Url::parse("https://huggingface.co/api/whoami-v2")?)
            .await?
            .json()
            .await?;
        Ok(v["name"]
            .as_str()
            .context("Missing account name")?
            .to_owned())
    }
    pub async fn search(&self, query: &str, sort: &str, task: &str) -> Result<Vec<Repository>> {
        let query = if query.starts_with("https://") {
            modelshelf_core::parse_repository(query)?
        } else {
            query.trim().to_owned()
        };
        let sort = match sort {
            "likes" | "lastModified" | "downloads" | "trendingScore" => sort,
            _ => "downloads",
        };
        let mut url = Url::parse("https://huggingface.co/api/models")?;
        url.query_pairs_mut()
            .append_pair("search", &query)
            .append_pair("sort", sort)
            .append_pair("direction", "-1")
            .append_pair("limit", "48")
            .append_pair("full", "true");
        if !task.is_empty() {
            url.query_pairs_mut().append_pair("pipeline_tag", task);
        }
        let values: Vec<Value> = self.fetch(url).await?.json().await?;
        Ok(values.into_iter().map(repository).collect())
    }
    pub async fn details(&self, repo: &str, revision: &str) -> Result<Repository> {
        let repo = modelshelf_core::parse_repository(repo)?;
        let mut url = Url::parse(&format!(
            "https://huggingface.co/api/models/{repo}/revision/"
        ))?;
        url.path_segments_mut()
            .unwrap()
            .pop_if_empty()
            .push(if revision.is_empty() {
                "main"
            } else {
                revision
            });
        let mut result = repository(self.fetch(url).await?.json().await?);
        if result.sha.len() != 40 || !result.sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("Provider did not return an immutable commit identity");
        }
        let mut next = Some(Url::parse(&format!(
            "https://huggingface.co/api/models/{repo}/tree/{}?recursive=true&expand=false",
            result.sha
        ))?);
        while let Some(url) = next.take() {
            let response = self.fetch(url).await?;
            next = response
                .headers()
                .get("link")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').find(|p| p.contains("rel=\"next\"")))
                .and_then(|s| s.split('<').nth(1)?.split('>').next())
                .map(Url::parse)
                .transpose()?;
            if let Some(url) = &next {
                if url.host_str() != Some("huggingface.co")
                    || !url.path().starts_with(&format!("/api/models/{repo}/tree/"))
                {
                    bail!("Invalid pagination origin");
                }
            }
            let entries: Vec<Value> = response.json().await?;
            for entry in entries {
                if entry["type"] == "file" {
                    let path = entry["path"]
                        .as_str()
                        .context("Missing file path")?
                        .to_owned();
                    modelshelf_core::storage::validate_relative(&path)?;
                    result.files.push(RemoteFile {
                        path,
                        size: entry["size"].as_u64().context("Missing file size")?,
                        sha256: entry["lfs"]["oid"]
                            .as_str()
                            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                            .map(str::to_owned),
                    });
                }
            }
        }
        if let Some(file) = result
            .files
            .iter()
            .find(|f| f.path == "README.md" && f.size <= 2_000_000)
        {
            let mut response = self
                .fetch(file_url(&repo, &result.sha, &file.path)?)
                .await?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                if bytes.len().saturating_add(chunk.len()) > 2_000_000 {
                    bail!("Model card exceeds the safe display limit");
                }
                bytes.extend_from_slice(&chunk);
            }
            result.readme = String::from_utf8_lossy(&bytes).into_owned();
        }
        let refs: Value = self
            .fetch(Url::parse(&format!(
                "https://huggingface.co/api/models/{repo}/refs"
            ))?)
            .await?
            .json()
            .await?;
        for key in ["branches", "tags"] {
            if let Some(refs) = refs[key].as_array() {
                result.revisions.extend(
                    refs.iter()
                        .filter_map(|v| v["name"].as_str())
                        .map(str::to_owned),
                );
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origins_and_encoding() {
        assert!(!allowed(&Url::parse("https://localhost/x").unwrap()));
        assert!(!allowed(
            &Url::parse("https://huggingface.co.evil.test").unwrap()
        ));
        assert!(allowed(
            &Url::parse("https://cas-bridge.xethub.hf.co/file").unwrap()
        ));
        assert!(file_url("a/b", "main", "../escape").is_err());
        assert!(file_url("a/b", "main", "folder/model.gguf")
            .unwrap()
            .as_str()
            .contains("/resolve/main/folder/model.gguf"));
    }
}
