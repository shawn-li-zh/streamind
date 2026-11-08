import type { ToolCallStatus } from '@/api/modules/enums.ts'
import type { SectionNames } from '@/views/chat/index.ts'
import type { AgentResponseSection } from '@/api/modules/event.ts'

export type SsePayload = SectionEventPayload | EnvEventPayload | ToolCallEventPayload

export type SectionEventPayload = { event: 'section'; bind_event_id: string } & (
  | SectionStartOrEnd
  | SectionOpen
  | SectionChunk
  | SectionClose
)

export interface SectionStartOrEnd {
  type: 'start' | 'end'
  data: string
}

export interface SectionOpen {
  type: 'open'
  data: SectionNames
}

export interface SectionChunk {
  type: 'chunk'
  data: {
    tag: SectionNames
    chunk: string
  }
}

export interface SectionClose {
  type: 'close'
  data: AgentResponseSection
}

//============环境======
export type EnvEventPayload = { event: 'env'; bind_event_id: string } & (
  | TimerEvent
  | ErrorEvent
  | UserRequestEvent
)

export interface TimerEvent {
  type: 'timer'
  data: { attach: string; source_event_id: string }
}

export interface ErrorEvent {
  type: 'error'
  data: string
}

export interface UserRequestEvent {
  type: 'userRequest'
  data: {
    content: string
    attachments: string[]
  }
}

//========工具调用=========
export type ToolCallEventPayload = {
  event: 'tool_call'
  bind_event_id: string
} & (ToolCallStart | ToolCallActive | ToolCallSuccess | ToolCallFailure | ToolCallEnd)

export interface ToolCallStart {
  type: 'start'
  data: {
    id: string
    package: string
    tool: string
    args: Record<string, any>
  }[]
}

export interface ToolCallActive {
  type: 'active'
  data: string
}

export interface ToolCallResult {
  id: string
  values: {
    type: 'raw' | 'json' | 'image'
    payload: any
  }[]
  elapsed: string
  status: ToolCallStatus
}

export interface ToolCallSuccess {
  type: 'success'
  data: ToolCallResult
}

export interface ToolCallFailure {
  type: 'failure'
  data: ToolCallResult
}

export interface ToolCallEnd {
  type: 'end'
  data: void
}

//==========
