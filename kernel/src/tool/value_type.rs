pub enum ToolValueType {
    Bool,
    Number,
    String,
    Enum(Vec<&'static str>),
    //类型名
    Object(&'static str),
    Map(Box<ToolValueType>),
    Array(Box<ToolValueType>),
    Any,
}
impl ToolValueType {
    pub fn to_doc(&self) -> String {
        match self {
            ToolValueType::Bool => "boolean".to_string(),
            ToolValueType::Number => "number".to_string(),
            ToolValueType::String => "string".to_string(),
            ToolValueType::Enum(items) => {
                let arr: Vec<String> = items.iter().map(|v| format!(r#""{}""#, v)).collect();
                arr.join("|")
            }
            ToolValueType::Object(ty) => ty.to_string(),
            ToolValueType::Map(ty) => format!("Record<string,{}>", ty.to_doc()),
            ToolValueType::Array(ty) => format!("{}[]", ty.to_doc()),
            ToolValueType::Any => "any".to_string(),
        }
    }
}
pub struct TypeInfo {
    pub name: &'static str,
    pub annotation: &'static str,
    pub fields: Vec<TypeItemInfo>,
}
impl TypeInfo {
    pub fn to_doc(&self) -> String {
        let mut doc = String::new();
        if !self.annotation.is_empty() {
            doc.push_str("// ");
            doc.push_str(self.annotation);
            doc.push_str("\n");
        }
        doc.push_str("interface ");
        doc.push_str(self.name);
        doc.push_str(" {\n");
        for field in &self.fields {
            doc.push_str(&field.to_doc(3));
            doc.push_str("\n");
        }
        doc.push_str("}");
        doc
    }
}
pub struct TypeItemInfo {
    pub name: &'static str,
    pub annotation: &'static str,
    pub value_type: ToolValueType,
    pub is_option: bool,
}
impl TypeItemInfo {
    pub fn to_type_ident(&self) -> String {
        let mut doc = String::new();
        doc.push_str(self.name);
        if self.is_option {
            doc.push_str("?");
        }
        doc.push_str(": ");
        doc.push_str(&self.value_type.to_doc());
        doc
    }
    pub fn to_doc(&self, space: usize) -> String {
        let mut doc = String::new();
        let space = " ".repeat(space);
        if !self.annotation.is_empty() {
            doc.push_str(&space);
            doc.push_str("// ");
            doc.push_str(self.annotation);
            doc.push_str("\n");
        }
        doc.push_str(&space);
        doc.push_str(&self.to_type_ident());
        doc
    }
}
