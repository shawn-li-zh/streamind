import requestHttp from '@/api'

export interface Entity {
  id: string
  agent_id: string
  component_id: string
  response_event_id:string,
  section_index:number,
  title: string
  state: any
  class: string
  style: string
}

export function getById(id: string) {
  return requestHttp.get<Entity>(`/api/render_state/get/${id}`)
}

export function get_by_pos(response_event_id: string, section_index: number) {
  return requestHttp.get<Entity>(`/api/render_state/get_by_pos`, {
    response_event_id,
    section_index,
  })
}

export function set_state_by_id(id: string, state: any) {
  return requestHttp.post<Entity>(`/api/render_state/state/{id}`, state)
}
export function update_value(id: string, state: any) {
  return requestHttp.post<Entity>(`/api/render_state/state/{id}`, state)
}
