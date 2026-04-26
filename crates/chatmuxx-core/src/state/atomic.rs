use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::{ChatMuxXError, IoContext, Result},
    state::files,
};

pub async fn load_json_or_default<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned + Default,
{
    match tokio::fs::read_to_string(path).await {
        Ok(text) => serde_json::from_str(&text).map_err(|source| ChatMuxXError::JsonDeserialize {
            path: path.to_path_buf(),
            source,
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(source) => Err(ChatMuxXError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub async fn save_json<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|source| ChatMuxXError::JsonSerialize {
            path: path.to_path_buf(),
            source,
        })?;

    if let Some(parent) = path.parent() {
        files::ensure_state_dir(parent).await?;
    }

    let tmp_path = path.with_extension("tmp");
    tokio::fs::write(&tmp_path, bytes).await.at(&tmp_path)?;
    tokio::fs::rename(&tmp_path, path).await.at(path)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
    struct Sample {
        schema_version: u32,
        value: String,
    }

    #[tokio::test]
    async fn missing_json_loads_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("missing.json");

        let sample: Sample = load_json_or_default(&path).await.expect("load default");

        assert_eq!(sample, Sample::default());
    }

    #[tokio::test]
    async fn save_and_load_json_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sample.json");
        let sample = Sample {
            schema_version: 1,
            value: "hello".to_owned(),
        };

        save_json(&path, &sample).await.expect("save json");
        let loaded: Sample = load_json_or_default(&path).await.expect("load json");

        assert_eq!(loaded, sample);
    }
}
