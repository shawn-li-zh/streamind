import requestHttp from '@/api'

export interface Entity {
  id: string
  agent_id: string
  pos: number
  dumped: boolean
  title: string
  content: string
  time: string
}

export function list(agent_id: string) {
  return requestHttp.get<Entity[]>('/api/state/list', { agent_id })
}
