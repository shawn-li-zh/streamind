import requestHttp from '@/api'
import type { EventAndSections } from '@/stores/agent.ts'

export type SectionName = 'think' | 'reply' | 'render' | 'tool_call' | 'interrupt' | 'push'|"reset"|"pop"

export interface Entity {
  id: string
  agent_id: string
  event_id: string
  name: SectionName
  content: string
  payload: any
  value?: any
  elapsed: string
}

export interface RenderPayload {
  name: string
  title: string
  path: string
  class: string
  style: string
  attrs: any
}

export interface PushPayload {
  id: string
  pos: number
  title: string
  content: string
}

export function list_by_event_id(agent_id: string, event_id: string) {
  return requestHttp.get<Entity[]>('/api/section/list', { agent_id, event_id })
}

export function get_by_id(id: string) {
  return requestHttp.get<Entity>(`/api/section/${id}`)
}

export function list_in_range(agent_id: string, start: string, end: string, names?: SectionName[]) {
  return requestHttp.get<EventAndSections[]>('/api/section/list_in_range', {
    agent_id,
    start,
    end,
    names,
  })
}

export function update_value(id: string, value: any) {
  return requestHttp.post<Entity>(`/api/section/update_value/${id}`, {
    value,
  })
}
