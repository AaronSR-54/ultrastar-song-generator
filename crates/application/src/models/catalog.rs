use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPurpose {
    Separation,
    Transcription,
    Pitch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: String,
    pub name: String,
    pub purpose: ModelPurpose,
    pub filename: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub license: String,
    pub requirements: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalog {
    pub schema_version: u32,
    pub models: Vec<ModelSpec>,
}

impl ModelCatalog {
    pub fn embedded() -> Result<Self, CatalogError> {
        Self::from_toml(include_str!("../../assets/models.toml"))
    }

    pub fn from_toml(input: &str) -> Result<Self, CatalogError> {
        let catalog: Self = toml::from_str(input)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn get(&self, id: &str) -> Option<&ModelSpec> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn validate(&self) -> Result<(), CatalogError> {
        if self.schema_version != 1 {
            return Err(CatalogError::Invalid(format!(
                "unsupported catalog schema {}",
                self.schema_version
            )));
        }
        if self.models.is_empty() {
            return Err(CatalogError::Invalid("catalog is empty".into()));
        }

        let mut ids = BTreeSet::new();
        let mut filenames = BTreeSet::new();
        for model in &self.models {
            if model.id.trim().is_empty() {
                return Err(CatalogError::Invalid("model id is empty".into()));
            }
            if !ids.insert(model.id.clone()) {
                return Err(CatalogError::Duplicate(model.id.clone()));
            }
            if model.name.trim().is_empty() {
                return Err(CatalogError::Invalid(format!(
                    "{}: name is empty",
                    model.id
                )));
            }
            if model.size_bytes == 0 {
                return Err(CatalogError::Invalid(format!(
                    "{}: size must be positive",
                    model.id
                )));
            }
            if model.sha256.len() != 64
                || !model.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(CatalogError::Invalid(format!(
                    "{}: sha256 must be 64 hexadecimal characters",
                    model.id
                )));
            }
            if !model.url.starts_with("https://") {
                return Err(CatalogError::Invalid(format!(
                    "{}: model URLs must use HTTPS",
                    model.id
                )));
            }
            if model.license.trim().is_empty() {
                return Err(CatalogError::Invalid(format!(
                    "{}: license is empty",
                    model.id
                )));
            }
            if model.requirements.trim().is_empty() {
                return Err(CatalogError::Invalid(format!(
                    "{}: requirements are empty",
                    model.id
                )));
            }
            if model.filename.trim().is_empty()
                || model.filename.contains(['/', '\\'])
                || model.filename.contains("..")
                || model.filename.starts_with('.')
            {
                return Err(CatalogError::Invalid(format!(
                    "{}: unsafe filename {:?}",
                    model.id, model.filename
                )));
            }
            if !filenames.insert(model.filename.clone()) {
                return Err(CatalogError::Duplicate(model.filename.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("invalid catalog TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("invalid catalog: {0}")]
    Invalid(String),
    #[error("duplicate model id or filename: {0}")]
    Duplicate(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_toml() -> String {
        let hash = "a".repeat(64);
        format!(
            r#"
schema_version = 1

[[models]]
id = "model"
name = "Model"
purpose = "separation"
filename = "model.onnx"
url = "https://example.com/model.onnx"
size_bytes = 10
sha256 = "{hash}"
license = "MIT"
requirements = "CPU"
"#
        )
    }

    #[test]
    fn embedded_catalog_is_valid_and_contains_expected_models() {
        let catalog = ModelCatalog::embedded().unwrap();
        assert!(catalog.get("htdemucs").is_some());
        assert!(catalog.get("whisper-large-v3-turbo-q5").is_some());
        assert_eq!(catalog.models.len(), 2);
    }

    #[test]
    fn parses_a_valid_catalog() {
        let catalog = ModelCatalog::from_toml(&valid_toml()).unwrap();
        assert_eq!(catalog.models[0].id, "model");
    }

    #[test]
    fn rejects_non_https_urls() {
        let toml = valid_toml().replace("https://example.com", "http://example.com");
        assert!(matches!(
            ModelCatalog::from_toml(&toml),
            Err(CatalogError::Invalid(_))
        ));
    }

    #[test]
    fn rejects_bad_hashes_and_unsafe_filenames() {
        let bad_hash = valid_toml().replace(&"a".repeat(64), "not-a-hash");
        assert!(matches!(
            ModelCatalog::from_toml(&bad_hash),
            Err(CatalogError::Invalid(_))
        ));

        let traversal =
            valid_toml().replace("filename = \"model.onnx\"", "filename = \"../model.onnx\"");
        assert!(matches!(
            ModelCatalog::from_toml(&traversal),
            Err(CatalogError::Invalid(_))
        ));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let duplicated = format!(
            "{}\n[[models]]\nid = \"model\"\nname = \"Other\"\npurpose = \"transcription\"\nfilename = \"other.bin\"\nurl = \"https://example.com/other.bin\"\nsize_bytes = 10\nsha256 = \"{}\"\nlicense = \"MIT\"\nrequirements = \"CPU\"\n",
            valid_toml(),
            "b".repeat(64)
        );
        let result = ModelCatalog::from_toml(&duplicated);
        assert!(matches!(result, Err(CatalogError::Duplicate(_))));
    }
}
