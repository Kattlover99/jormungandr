mod duration;
mod iteration;
use assert_fs::fixture::PathChild;
pub use duration::DurationBasedClientLoad;
pub use iteration::IterationBasedClientLoad;

use crate::common::{
    file_utils,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter, StartupError},
};
use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use std::{fs, result::Result};

use super::ClientLoadConfig;
use assert_fs::TempDir;

pub fn copy_initial_storage_if_used(
    config: &ClientLoadConfig,
    storage_folder: &str,
    temp_dir: &TempDir,
) {
    if let Some(storage) = &config.initial_storage {
        let client_storage: PathBuf = temp_dir.child(storage_folder.to_string()).path().into();
        if client_storage.exists() {
            fs::remove_dir_all(&client_storage).expect("cannot remove existing client storage");
        }
        fs::create_dir(&client_storage).expect("cannot create client storage");
        file_utils::copy_folder(storage, &client_storage, true);
    }
}

pub fn start_node(
    client_config: &ClientLoadConfig,
    storage_folder_name: &str,
    temp_dir: &TempDir,
) -> Result<JormungandrProcess, StartupError> {
    copy_initial_storage_if_used(client_config, &storage_folder_name, temp_dir);

    let config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![client_config.trusted_peer()])
        .with_block_hash(client_config.block0_hash.to_string())
        .with_storage(&temp_dir.child(storage_folder_name.to_string()))
        .build(temp_dir);

    Starter::new().config(config).passive().start_async()
}

pub struct ScenarioProgressBar {
    progress_bar: ProgressBar,
}

impl ScenarioProgressBar {
    pub fn new(progress_bar: ProgressBar, prefix: &str) -> Self {
        let spinner_style = ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{prefix:.bold.dim} {spinner} {wide_msg}");
        progress_bar.set_style(spinner_style.clone());
        progress_bar.set_prefix(prefix);
        progress_bar.set_message(&format!("initializing..."));

        Self { progress_bar }
    }

    pub fn set_progress(&self, progress: &str) {
        self.progress_bar
            .set_message(&format!("bootstrapping... {}", progress));
    }

    pub fn set_error_lines(&self, iter: Vec<String>) {
        for line in iter {
            self.progress_bar.set_message(&format!("Error: {}", line));
        }
    }

    pub fn set_finished(&self) {
        self.progress_bar
            .set_message(&format!("bootstrapped succesfully."));
    }
}
