use std::{io::Write, path::PathBuf};

use chrono::Local;
use kernel::{agent::event::EnvEvent, message::Role, section::Section};
use runtime::agent::{event::AppEnvEvent, event_context::MindNode};
use tokio::{
    fs::{self, File},
    io::{self, AsyncWriteExt as _},
};

pub struct AgentLogger {
    log_file: tokio::sync::Mutex<File>,
}
impl AgentLogger {
    pub async fn new(worksapce: &PathBuf) -> io::Result<Self> {
        let time = Local::now().format("%Y-%m-%d %H_%M").to_string();
        let path = worksapce.join(format!("log/{}.txt", &time));
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        let log_file = File::options()
            .write(true)
            .append(true)
            .create(true)
            .create_new(true)
            .open(path)
            .await?;
        Ok(Self {
            log_file: tokio::sync::Mutex::new(log_file),
        })
    }
    pub async fn log_event(&self, event: &AppEnvEvent) -> io::Result<()> {
        let msg = event.to_message();
        let role = msg.role();
        let (text, _) = msg.to_tuple();
        let s: String = format!("\n{:?}:\n{}\n", role, text);
        {
            let mut log_file = self.log_file.lock().await;
            log_file.write(s.as_bytes()).await?;
            log_file.flush().await?;
        }
        Ok(())
    }

    pub async fn log_message(&self, msg: &str) -> io::Result<()> {
        let time = Local::now().format("%Y-%m-%d %H:%M:%s").to_string();
        let s = format!("\n<log time='{time}'>\n{}\n</log>", msg);
        let mut log_file = self.log_file.lock().await;
        log_file.write(s.as_bytes()).await?;
        log_file.flush().await?;
        Ok(())
    }
}
