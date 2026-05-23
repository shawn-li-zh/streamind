use async_trait::async_trait;

// pub struct Hook {
//     pub name: &'static str,
//     pub func: Box<dyn Fn(&[&str]) -> String + Send + Sync + 'static>,
// }
// #[async_trait]
// pub trait RuleHook {
//     async fn execute(&self, args: Vec<serde_json::Value>) -> String;
// }




pub struct HookRegistry {
    hooks: std::collections::HashMap<String, fn(&[&str]) -> String>,
}

impl HookRegistry {
    pub fn new() -> Self {
        let mut registry = HookRegistry {
            hooks: std::collections::HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        self.register("inject", inject);
        self.register("recall", recall);
        self.register("context", context);
        self.register("summarize", summarize);
        self.register("store", store);
        self.register("search", search);
    }

    pub fn register(&mut self, name: &str, func: fn(&[&str]) -> String) {
        self.hooks.insert(name.to_string(), func);
    }

    pub fn call(&self, name: &str, args: &[&str]) -> Option<String> {
        self.hooks.get(name).map(|f| f(args))
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn inject(args: &[&str]) -> String {
    if args.is_empty() {
        return String::new();
    }
    format!("[INJECT] {}", args.join(" "))
}

fn recall(args: &[&str]) -> String {
    if args.is_empty() {
        return String::new();
    }
    let keyword = args.join(" ");
    format!("[RECALL] 回忆: 关于\"{}\"的信息已注入上下文", keyword)
}

fn context(args: &[&str]) -> String {
    if args.is_empty() {
        return String::new();
    }
    format!("[CONTEXT] {}", args.join(" "))
}

fn summarize(args: &[&str]) -> String {
    if args.is_empty() {
        return String::new();
    }
    format!("[SUMMARIZE] 已总结: {}", args.join(" "))
}

fn store(args: &[&str]) -> String {
    if args.is_empty() {
        return String::new();
    }
    format!("[STORE] 已存储: {}", args.join(" "))
}

fn search(args: &[&str]) -> String {
    if args.is_empty() {
        return String::new();
    }
    let keyword = args.join(" ");
    format!("[SEARCH] 搜索结果: 找到与\"{}\"相关的内容", keyword)
}

pub struct BuiltinHooks;

impl BuiltinHooks {
    pub fn create_engine() -> crate::rule::engine::RuleEngine {
        let mut engine = crate::rule::engine::RuleEngine::new();

        engine.register_hook("inject", |args: &str| {
            if args.is_empty() {
                return String::new();
            }
            format!("[INJECT] {}", args)
        });

        engine.register_hook("recall", |args: &str| {
            if args.is_empty() {
                return String::new();
            }
            format!("[RECALL] 回忆: 关于\"{}\"的信息已注入上下文", args)
        });

        engine.register_hook("context", |args: &str| {
            if args.is_empty() {
                return String::new();
            }
            format!("[CONTEXT] {}", args)
        });

        engine.register_hook("summarize", |args: &str| {
            if args.is_empty() {
                return String::new();
            }
            format!("[SUMMARIZE] 已总结: {}", args)
        });

        engine.register_hook("store", |args: &str| {
            if args.is_empty() {
                return String::new();
            }
            format!("[STORE] 已存储: {}", args)
        });

        engine.register_hook("search", |args: &str| {
            if args.is_empty() {
                return String::new();
            }
            format!("[SEARCH] 搜索结果: 找到与\"{}\"相关的内容", args)
        });

        engine
    }
}
