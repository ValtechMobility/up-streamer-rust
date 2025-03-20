use async_trait::async_trait;
use clap::Command;
use log::info;
use serde_json::{json, Value};
use std::{
    fmt,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::Arc,
};
use tempdir::TempDir;
use tokio::sync::Mutex;
use up_rust::{UListener, UMessage};

pub(crate) const SERVICE_URI: &str = "service/1/1/1";
pub(crate) const CLIENT_URI: &str = "client/1/1/0";

#[derive(Debug)]
pub enum TestError {
    Io(std::io::Error),
    Config(String),
    Process(String),
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestError::Io(e) => write!(f, "IO error: {}", e),
            TestError::Config(msg) => write!(f, "Configuration error: {}", msg),
            TestError::Process(msg) => write!(f, "Process error: {}", msg),
        }
    }
}

impl From<std::io::Error> for TestError {
    fn from(err: std::io::Error) -> Self {
        TestError::Io(err)
    }
}
impl std::error::Error for TestError {}

pub(crate) struct MessageCollector {
    id: i32,
    pub(crate) messages: Arc<Mutex<Vec<UMessage>>>,
}

impl MessageCollector {
    pub(crate) fn new(id: i32) -> Self {
        Self {
            id,
            messages: Arc::new(Mutex::new(vec![])),
        }
    }

    pub(crate) async fn add_message(&self, message: UMessage) {
        let mut messages = self.messages.lock().await;
        messages.push(message);
    }

    pub(crate) async fn get_messages(&self) -> Vec<UMessage> {
        let messages = self.messages.lock().await;
        messages.clone()
    }
}

#[async_trait]
impl UListener for MessageCollector {
    async fn on_receive(&self, msg: UMessage) {
        info!("MessageCollector {}: Received a message: {msg:?}", self.id);
        self.add_message(msg).await
    }
}

pub(crate) struct StreamerInstance {
    process: Child,
    _config_dir: TempDir, // TempDir is dropped (and cleaned up) when StreamerInstance is dropped
}

impl StreamerInstance {
    pub(crate) fn new(test_id: u16) -> Self {
        // Create temporary directory for config files
        let config_dir = TempDir::new(&test_id.to_string()).unwrap();
    }
}

fn create_test_config(test_id: u16, config_dir: &TempDir) -> Result<(), TestError> {
    let main_config_path = config_dir.path().join("CONFIG.json5");
}

fn create_main_config(test_id: u16) -> String {
    let config = json!({
        "up_streamer_config": {
            "message_queue_size": 10000
        },
        "streamer_uuri": {
            "authority": format!("test_authority_{}", test_id),
            "ue_id": test_id,
            "ue_version_major": 1
        },
        "usubscription_config": {
            "file_path": "subscription_data.json"
        },
        "transports": {
            "zenoh": [
                {
                    "config_file": "ZENOH_CONFIG_1.json5",
                    "authority": "authority_B",
                    "endpoint": format!("endpoint_zenoh_{}", test_id),
                    "forwarding": [format!("endpoint_mqtt_{}", test_id)]
                }
            ],
            "mqtt": [
                {
                    "config_file": "MQTT_CONFIG.json5",
                    "authority": "authority_A",
                    "endpoint": format!("endpoint_mqtt_{}", test_id),
                    "forwarding": [format!("endpoint_zenoh_{}", test_id)]
                }
            ]
        }
    });

    config.to_string()
}

fn get_streamer_binary_path() -> Result<PathBuf, TestError> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let binary_path = Path::new(manifest_dir).join("../target/debug/configurable-streamer");

    if !binary_path.exists() {
        Command::new("cargo run --package configurable-streamer").build();
    }

    Ok(binary_path)
}
