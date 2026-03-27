use std::{cell::RefCell, rc::Rc};

use async_openai::types::ChatCompletionRequestMessage;
//入口控制

pub enum StreamCommand {
    Next(ShareInlet),
    Append(ChatCompletionRequestMessage),
    Out(String),
    Stop(),
}

//流的入口
pub type ShareInlet = Rc<RefCell<dyn StreamInlet>>;
pub trait StreamInlet {
    fn input(&mut self, chunk: &String) -> Vec<StreamCommand>;
    fn end(&mut self);
}

pub struct TagMatchInlet {
    tag: String,
    pub buffer: String,
    prefix: String,
    suffix: String,
    pub is_matched: bool,
    handler: Box<dyn FnMut(String, String) -> Vec<StreamCommand>>,
}
impl TagMatchInlet {
    pub fn new(tag: String, handler: Box<dyn FnMut(String, String) -> Vec<StreamCommand>>) -> Self {
        let prefix = format!("<{} ", &tag);
        let suffix = format!("</{}>", &tag);
        Self {
            tag,
            buffer: String::new(),
            prefix,
            suffix,
            is_matched: false,
            handler,
        }
    }
    pub fn share(self) -> ShareInlet {
        Rc::new(RefCell::new(self))
    }
}
impl StreamInlet for TagMatchInlet {
    fn input<'a>(&'a mut self, chunk: &String) -> Vec<StreamCommand> {
        self.buffer.push_str(chunk);
        let mut commands = vec![];
        if self.is_matched {
            //已匹配到tag，继续寻找结束标签
            if let Some(end) = self.buffer.rfind(&self.suffix) {
                let end_i = end + self.suffix.len();
                let content = self.buffer[..end_i].to_string();
                let extra = self.buffer[end_i..].to_string();
                return (self.handler)(content, extra);
            }
            return commands;
        }
        //检查是否包含prefix,并得到开始的index
        if let Some(start) = self.buffer.find(&self.prefix) {
            let out = self.buffer[..start].to_string();
            commands.push(StreamCommand::Out(out));
            self.is_matched = true;
        } else {
            //未匹配到标签，保留buffer的最后部分，防止标签被切断，其他的全部输出
            let reserve = self.prefix.len() - 1;
            if self.buffer.len() > reserve {
                let out = self.buffer[..self.buffer.len() - reserve].to_string();
                commands.push(StreamCommand::Out(out));
                self.buffer = self.buffer[self.buffer.len() - reserve..].to_string();
            }
        }
        commands
    }

    fn end(&mut self) {
        todo!()
    }
}
