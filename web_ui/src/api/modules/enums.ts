
export enum EventTypeEnum {
  AgentResponse = 'agentResponse',
  UserRequest = 'userRequest',
  Interrupt = 'interrupt',
  Interaction = 'interaction',
  ToolResponse = 'toolResponse',
  Timer = 'timer',
  Error = 'error',
  Heartbeat = 'heartbeat',
  Reflection = 'reflection',
}

export enum EventStatusEnum {
  Pending = 'pending',
  Processing = 'processing',
  Done = 'done',
  Unloaded = 'unloaded',
}

export enum AgentStatusEnum {
  Idle = 'idle', //空闲
  Sleep = 'sleep',      //睡眠
  Processing = 'processing', //处理任务中
  Paused = 'paused',     //暂停 需要用户手动恢复到空闲
}

export enum ToolCallStatus {
  Undo = 'undo',
  Active = 'active',
  Success = 'success',
  Failure = 'failure',
  Timeout = 'timeout',
}
