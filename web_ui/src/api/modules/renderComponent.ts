import requestHttp from '@/api'

export interface Entity {
  id: string
  name: string
  path: string
  annotation: string
  doc: string
  authorType: 'system' | 'official' | 'agent' | 'self' | 'others'
  authorName: string
  preview_attrs:any
  create_time: string
}

export function getById(id: string) {
  return requestHttp.get<Entity>(`/api/render_component/${id}`)
}

export function page(page: number, size: number, key?: string) {
  return requestHttp.get<Entity[]>(`/api/render_component/page`, { size, page, key })
}

export function total(key?: string) {
  return requestHttp.get<number>(`/api/render_component/total`, { key })
}

export function create(data: Partial<Entity>) {
  return requestHttp.post<string>(`/api/render_component`, data)
}

export function update(id: string, data: Partial<Entity>) {
  return requestHttp.put<void>(`/api/render_component/${id}`, data)
}
