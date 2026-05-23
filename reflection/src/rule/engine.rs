// use regex::Regex;
// use std::collections::HashMap;

// use crate::rule::ast;
// pub trait Judger {
    
// }
// pub struct RuleEngine {
//     hooks: HashMap<String, Box<dyn Fn(&str) -> String + Send + Sync>>,
// }

// impl RuleEngine {
//     pub fn new() -> Self {
//         RuleEngine {
//             hooks: HashMap::new(),
//         }
//     }

//     pub fn register_hook<F>(&mut self, name: &str, hook: F)
//     where
//         F: Fn(&str) -> String + Send + Sync + 'static,
//     {
//         self.hooks.insert(name.to_string(), Box::new(hook));
//     }

//     pub fn execute_hook(&self, hook_call: &str) -> Option<String> {
//         let parts: Vec<&str> = hook_call.splitn(2, '(').collect();
//         if parts.len() != 2 {
//             return None;
//         }
//         let name = parts[0];
//         let mut args = parts[1].to_string();
//         if args.ends_with(')') {
//             args.pop();
//         }
//         let args = args.trim();

//         if let Some(hook) = self.hooks.get(name) {
//             Some(hook(args))
//         } else {
//             None
//         }
//     }

//     pub fn evaluate_rule(&self, expr: &str, text: &str) -> bool {
//         ast::eval(text, exprs)
//         if let Some(expr) = parse_rule(expr) {
//             expr.evaluate(text)
//         } else {
//             false
//         }
//     }

//     pub fn evaluate_and_inject(&self, expr: &str, hook: &str, text: &str) -> Option<String> {
//         if !self.evaluate_rule(expr, text) {
//             return None;
//         }
//         self.execute_hook(hook)
//     }
// }

// impl Default for RuleEngine {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[test]
//     fn test_parse_rule() {
//         let expr = "contains(foo)";
//         let expr = parse_rule(expr);
//         assert!(expr.is_some());
//         let expr = expr.unwrap();
//         assert_eq!(expr, Expr::Contains("foo".to_string()));
//     }
// }
