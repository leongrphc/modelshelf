use anyhow::{ensure, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeProfile {
    pub kind: String,
    pub data_directory: Option<PathBuf>,
}

impl RuntimeProfile {
    pub fn from_environment() -> Result<Self> {
        let requested = std::env::var("MODELSHELF_PROFILE").ok();
        let test_directory = std::env::var_os("MODELSHELF_TEST_DATA_DIR").map(PathBuf::from);
        Self::resolve(
            requested.as_deref(),
            cfg!(debug_assertions),
            test_directory.as_deref(),
        )
    }

    fn resolve(
        requested: Option<&str>,
        debug: bool,
        test_directory: Option<&Path>,
    ) -> Result<Self> {
        let kind = requested.unwrap_or(if debug { "development" } else { "production" });
        ensure!(
            matches!(kind, "production" | "development" | "test"),
            "MODELSHELF_PROFILE must be production, development, or test"
        );
        ensure!(
            !debug || kind != "production",
            "Debug builds cannot use the production profile"
        );
        ensure!(
            kind == "test" || test_directory.is_none(),
            "MODELSHELF_TEST_DATA_DIR requires MODELSHELF_PROFILE=test"
        );
        let data_directory = if kind == "test" {
            let directory = test_directory
                .ok_or_else(|| anyhow::anyhow!("Test profile requires MODELSHELF_TEST_DATA_DIR"))?;
            ensure!(
                directory.is_absolute(),
                "Test data directory must be absolute"
            );
            let directory = directory.canonicalize()?;
            let temp = std::env::temp_dir().canonicalize()?;
            ensure!(directory.is_dir() && directory != temp && directory.starts_with(&temp), "Test data directory must be an existing child directory of the OS temporary directory");
            Some(directory)
        } else {
            None
        };
        Ok(Self {
            kind: kind.into(),
            data_directory,
        })
    }

    pub fn identifier(&self) -> &'static str {
        match self.kind.as_str() {
            "production" => "io.modelshelf.desktop",
            "development" => "io.modelshelf.desktop.development",
            _ => "io.modelshelf.desktop.test",
        }
    }

    pub fn data_directory_for(&self, application_directory: PathBuf) -> PathBuf {
        self.data_directory.clone().unwrap_or(application_directory)
    }

    pub fn credential(&self) -> Option<keyring::Result<keyring::Entry>> {
        if self.kind == "test" {
            None
        } else {
            Some(keyring::Entry::new(self.identifier(), "huggingface"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_defaults_separate_data_and_credentials_without_migrating_production() {
        let production = RuntimeProfile::resolve(None, false, None).unwrap();
        let development = RuntimeProfile::resolve(None, true, None).unwrap();
        assert_eq!(production.identifier(), "io.modelshelf.desktop");
        assert_eq!(
            development.identifier(),
            "io.modelshelf.desktop.development"
        );
        assert!(production.data_directory.is_none());
        assert!(development.data_directory.is_none());
        assert!(RuntimeProfile::resolve(Some("production"), true, None).is_err());
    }

    #[test]
    fn invalid_or_ambiguous_profiles_fail_closed() {
        assert!(RuntimeProfile::resolve(Some("typo"), false, None).is_err());
        assert!(RuntimeProfile::resolve(Some("test"), false, None).is_err());
        assert!(RuntimeProfile::resolve(None, false, Some(Path::new("test"))).is_err());
        assert!(RuntimeProfile::resolve(Some("test"), false, Some(Path::new("relative"))).is_err());
        assert!(RuntimeProfile::resolve(Some("test"), false, Some(&std::env::temp_dir())).is_err());
    }

    #[test]
    fn test_profile_uses_explicit_temp_directory_and_never_opens_keyring() {
        let directory =
            std::env::temp_dir().join(format!("modelshelf-profile-test-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let profile = RuntimeProfile::resolve(Some("test"), false, Some(&directory)).unwrap();
        assert_eq!(
            profile.data_directory,
            Some(directory.canonicalize().unwrap())
        );
        assert_eq!(profile.identifier(), "io.modelshelf.desktop.test");
        assert!(profile.credential().is_none());
        std::fs::remove_dir(&directory).unwrap();
    }
    #[test]
    fn test_settings_writes_preserve_production_and_development_databases() {
        use modelshelf_core::Database;
        let root = std::env::temp_dir().join(format!(
            "modelshelf-db-isolation-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let production = RuntimeProfile::resolve(None, false, None).unwrap();
        let development = RuntimeProfile::resolve(None, true, None).unwrap();
        let fixture_paths: Vec<_> = [&production, &development]
            .iter()
            .map(|profile| {
                let directory = profile.data_directory_for(root.join(profile.identifier()));
                std::fs::create_dir(&directory).unwrap();
                let path = directory.join("modelshelf.db");
                let db = Database::open(&path).unwrap();
                let mut settings = db.settings().unwrap();
                settings.default_directory =
                    directory.join("user-models").to_string_lossy().into_owned();
                db.save_settings(&settings).unwrap();
                drop(db);
                (
                    path.clone(),
                    std::fs::read(&path).unwrap(),
                    settings.default_directory,
                )
            })
            .collect();
        let test_directory = root.join("automation");
        std::fs::create_dir(&test_directory).unwrap();
        let test = RuntimeProfile::resolve(Some("test"), false, Some(&test_directory)).unwrap();
        // Supplying the production fallback must still select the explicit test profile.
        let data = test.data_directory_for(root.join(production.identifier()));
        assert_eq!(data, test_directory.canonicalize().unwrap());
        let db = Database::open(&data.join("modelshelf.db")).unwrap();
        let mut settings = db.settings().unwrap();
        assert!(settings.default_directory.is_empty());
        settings.default_directory = data.join("models").to_string_lossy().into_owned();
        db.save_settings(&settings).unwrap();
        assert_eq!(
            db.settings().unwrap().default_directory,
            settings.default_directory
        );
        drop(db);
        for (path, original_bytes, original_directory) in &fixture_paths {
            assert_eq!(&std::fs::read(path).unwrap(), original_bytes);
            let fixture = Database::open(path).unwrap();
            assert_eq!(
                &fixture.settings().unwrap().default_directory,
                original_directory
            );
        }
        // Only this freshly created fixture tree is removed; no user directories are accessed.
        assert!(root
            .canonicalize()
            .unwrap()
            .starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
