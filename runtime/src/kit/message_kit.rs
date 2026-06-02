use async_openai::types::chat::ChatCompletionRequestMessage;

use kernel::message::{Information, RoleMessage};

use crate::kit::prompt_kit;

pub fn to_sdk_message(value: RoleMessage) -> ChatCompletionRequestMessage {
    let role = value.role();
    let (text, paths) = value.to_tuple();
    match role {
        kernel::message::Role::User => prompt_kit::user_message(text, paths),
        kernel::message::Role::System => prompt_kit::system_message(text),
        kernel::message::Role::Tool => prompt_kit::user_message(text, paths),
        kernel::message::Role::Assistant => prompt_kit::assistant_message(text),
    }
}

// pub fn to_assistant_ouput(sections: Vec<Section>) -> Option<String> {
//     if sections.is_empty() {
//         return None;
//     }
//     let has_tool_call = sections.iter().any(|s| s.name == "tool_call");
//     fn join(sections: Vec<Section>) -> String {
//         let mut text = String::new();
//         for section in sections {
//             text += &section.to_string();
//             text += "\n";
//         }
//         text
//     }
//     if has_tool_call {
//         Some(join(sections))
//     } else {
//         //去除思考内容
//         let sections: Vec<Section> = sections.into_iter().filter(|s| s.name != "think").collect();
//         if sections.is_empty() {
//             None
//         } else {
//             Some(join(sections))
//         }
//     }
// }

// pub fn to_assistant_ouput(sections: Vec<Section>) -> Vec<Information> {
//     sections
//         .into_iter()
//         .map(|s| {
//             let full = s.to_string();
//             Information::Function(s.name, full)
//         })
//         .collect()
// }
