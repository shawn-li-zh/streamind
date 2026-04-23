use std::{collections::HashMap, io::Write, path, sync::Arc};

use async_openai::types::ChatCompletionRequestMessage;
use chrono::Local;
use streamind_core::item::ItemGroup;

use crate::{
    agent::{event::AgentEvent, kit::prompt_kit, llm_engine::LLMEngine},
    global::{DATE_FMT, PROMPT_CACHE, TIME_FMT},
};
