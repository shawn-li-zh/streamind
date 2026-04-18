use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::global::{DB, PROMPT_CACHE};

pub async fn build_system_prompt(info: &entity::agent::Model) -> String {
    let items: Vec<entity::item::Model> = entity::item::Entity::find()
        .filter(entity::item::Column::AgentId.eq(info.id))
        .all(DB.get().unwrap())
        .await
        .unwrap();
    let mut memory = String::new();
    let mut focus = String::new();
    let mut goal = String::new();
    for item in items {
        let part = match item.group.as_str() {
            "memory" => &mut memory,
            "focus" => &mut focus,
            "goal" => &mut goal,
            _ => {
                continue;
            }
        };
        part.push_str("- ");
        part.push_str(&item.name);
        part.push_str(": ");
        part.push_str(&item.content);
        part.push_str("\n");
    }

    let generate_rule = PROMPT_CACHE.get("common/generate_rule").unwrap_or_default();
    let tool_call_rule = PROMPT_CACHE
        .get("common/tool_call_rule")
        .unwrap_or_default();

    format!(
        r#"
    # 基础信息:
    - 编号:{}
    - 名称:{}

    # 生成格式
    {}

    # 工具调用规则
    {}

    # 记忆
    {}

    # 关注
    {}

    # 目标
    {}
    "#,
        info.id, info.name, generate_rule, tool_call_rule, memory, focus, goal
    )
}
