import requestHttp from '@/api'
import { AgentStatusEnum } from '@/api/modules/enums.ts'
import { event } from '@/api/modules/index.ts'

export interface Entity {
  id: string
  model_id: string
  parent_id: string
  state_id: string
  name: string
  avatar: string
  persona: string
  workspace: string
  description: string
  status: AgentStatusEnum
  startup_time?: string
  children: Entity[]
}

export function get_by_id(id: string) {
  return requestHttp.get<Entity>(`/api/agent/${id}`)
}

export function tree() {
  return requestHttp.get<Entity[]>('/api/agent/tree')
}

export function update(agentId: string, data: Partial<Entity>) {
  return requestHttp.put<void>(`/api/agent/${agentId}`, data)
}

export function chat(agentId: string, content: string) {
  return requestHttp.post<void>(`/api/agent/chat/${agentId}`, {
    content,
    attachments: [],
  })
}

export function ui_interaction(
  agentId: string,
  body: {
    section_id: string
    message: string
  },
) {
  return requestHttp.post<void>(`/api/agent/ui_interaction/${agentId}`, body)
}

export function user_interrupt(agentId: string) {
  return requestHttp.post<void>(`/api/agent/user_interrupt/${agentId}`, {})
}

export function create(data: Omit<Entity, 'id' | 'children'>) {
  return requestHttp.post<string>('/api/agent', data)
}
