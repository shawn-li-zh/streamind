use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct PromptCache {
    templates: Arc<RwLock<HashMap<String, String>>>,
    watcher: Arc<RwLock<Option<RecommendedWatcher>>>,
    folder_path: PathBuf,
}

impl PromptCache {
    pub fn new(folder_path: PathBuf) -> Self {
        let cache = PromptCache {
            templates: Arc::new(RwLock::new(HashMap::new())),
            watcher: Arc::new(RwLock::new(None)),
            folder_path,
        };
        cache.load_templates();
        cache
    }

    pub fn from_str(folder_path: &str) -> Self {
        Self::new(PathBuf::from(folder_path))
    }

    fn load_templates(&self) {
        let mut templates = match self.templates.write() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to acquire write lock: {}", e);
                return;
            }
        };
        templates.clear();

        if let Ok(entries) = walkdir(&self.folder_path) {
            for (key, content) in entries {
                templates.insert(key, content);
            }
        }

        tracing::info!("Loaded {} templates", templates.len());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.templates.read().ok()?.get(key).cloned()
    }

    pub fn get_with_params(&self, key: &str, params: &HashMap<String, String>) -> Option<String> {
        let template = self.get(key)?;
        Some(fill_template(&template, params))
    }

    pub fn get_keys(&self) -> Vec<String> {
        self.templates
            .read()
            .ok()
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn reload(&self) {
        self.load_templates();
    }

    pub fn watch(&self) -> anyhow::Result<()> {
        let folder_path = self.folder_path.clone();
        let templates = self.templates.clone();
        let folder_path_for_reload = self.folder_path.clone();

        let (tx, mut rx) = mpsc::channel::<Event>(100);

        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        {
            let mut w = self.watcher.write().map_err(|e| anyhow::anyhow!("{}", e))?;
            *w = Some(watcher);
        }

        let mut watcher_guard = self.watcher.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        if let Some(ref mut w) = *watcher_guard {
            w.watch(&folder_path, RecursiveMode::Recursive)?;
        }
        drop(watcher_guard);

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                while let Some(event) = rx.recv().await {
                    if matches!(
                        event.kind,
                        notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                    ) {
                        tracing::info!("Template file changed, reloading...");
                        if let Ok(mut templates) = templates.write() {
                            templates.clear();
                            if let Ok(entries) = walkdir(&folder_path_for_reload) {
                                for (key, content) in entries {
                                    templates.insert(key, content);
                                }
                            }
                            tracing::info!("Reloaded {} templates", templates.len());
                        }
                    }
                }
            });
        });

        Ok(())
    }

    pub fn stop_watch(&self) {
        if let Ok(mut w) = self.watcher.write() {
            *w = None;
        }
    }
}

fn walkdir(folder: &PathBuf) -> anyhow::Result<Vec<(String, String)>> {
    let mut results = Vec::new();
    walkdir_recursive(folder, folder, &mut results)?;
    Ok(results)
}

fn walkdir_recursive(
    base: &PathBuf,
    current: &PathBuf,
    results: &mut Vec<(String, String)>,
) -> anyhow::Result<()> {
    let entries = std::fs::read_dir(current)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walkdir_recursive(base, &path, results)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let key = relative
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| {
                    relative
                        .parent()
                        .map(|p| {
                            if p.as_os_str().is_empty() {
                                s.to_string()
                            } else {
                                format!(
                                    "{}/{}",
                                    p.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                                    s
                                )
                            }
                        })
                        .unwrap_or_else(|| s.to_string())
                })
                .unwrap_or_default();

            let content = std::fs::read_to_string(&path)?;
            results.push((key, content));
        }
    }
    Ok(())
}

fn fill_template(template: &str, params: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\{(\w+)\}").unwrap();
    let mut result = template.to_string();
    for cap in re.captures_iter(template) {
        if let Some(key) = cap.get(1) {
            if let Some(value) = params.get(key.as_str()) {
                result = result.replace(&cap[0], value);
            }
        }
    }
    result
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new(PathBuf::from("prompts"))
    }
}
