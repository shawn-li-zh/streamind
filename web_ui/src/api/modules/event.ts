import { EventStatusEnum, EventTypeEnum, type ToolCallStatus } from '@/api/modules/enums.ts'
import requestHttp from '@/api'
import type { SectionNames } from '@/views/chat/index.ts'

export type Entity = {
  id: string
  agent_id: string
  state_id: string
  force: boolean
  time: string
} & (AgentResponseEvent | ToolResponseEvent | UserRequestEvent | InterruptEvent)

//======回复
export interface AgentResponseSection {
  id: string
  name: SectionNames
  content: string
  elapsed: string
}

export interface AgentResponseEvent {
  event_type: EventTypeEnum.AgentResponse
  data: AgentResponseSection[]
}

//===渲染
export interface RenderSectionData {
  renderStateId: string
  title: string
  class: string
  style: string
  state: any
}

//====
export interface ToolCallDetail {
  id: string
  call_id: string
  package: string
  tool: string
  values: {
    type: 'raw' | 'json' | 'image'
    payload: any
  }[]
  args: Record<string, any>
  elapsed: string
  status: ToolCallStatus
}

export interface ToolResponseEvent {
  event_type: EventTypeEnum.ToolResponse
  data: ToolCallDetail
}

//用户请求
export interface UserRequestEvent {
  event_type: EventTypeEnum.UserRequest
  data: { content: string; attachments: string[] }
}

//中断
export interface InterruptEvent {
  event_type: EventTypeEnum.Interrupt
  data: { reason: string }
}

export interface EventItem {
  id: string
  force: boolean
  time: string
  event_type: EventTypeEnum
  status: EventStatusEnum
}

export function list_items(agent_id: string, page: number, size: number, status?: EventStatusEnum) {
  return requestHttp.get<EventItem[]>('/api/event/list', { agent_id, page, size, status })
}

export function load(agent_id: string, size: number, event_id?: string, types?: EventTypeEnum[]) {
  return requestHttp.get<Entity[]>('/api/event/load', { agent_id, size, event_id, types })
}

export interface CountByStatus {
  status: EventStatusEnum
  count: number
}

export function count_by_status(agent_id: string) {
  return requestHttp.get<CountByStatus[]>(`/api/event/count_by_status/${agent_id}`, {})
}
