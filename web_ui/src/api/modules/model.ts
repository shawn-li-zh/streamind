import requestHttp from '@/api'

export interface Entity {
  id: string
  name: string
  api_url: string
  api_key: string
  input_modals: string[]
  output_modals: string[]
  consumed_tokens: number
  remark: string
  status: 'active' | 'inactive'
  cost: 'low' | 'medium' | 'high'
}

export function list() {
  return requestHttp.get<Entity[]>('/api/model/list', {  })
}

export function create(data: Omit<Entity, 'id' | 'consumed_tokens'>) {
  return requestHttp.post<string>('/api/model', data)
}

export function remove(id: string) {
  return requestHttp.delete<void>(`/api/model/${id}`)
}

export function update(id: string, data: Partial<Entity>) {
  return requestHttp.put<void>(`/api/model/${id}`, data)
}
