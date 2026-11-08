import requestHttp from '@/api'

export interface Entity {
  id: string
  path: string
  name: string
  version: string
}

export function get(id: string) {
  return requestHttp.get<Entity>(`/api/render_dependence/${id}`)
}

export function list(key?: string) {
  return requestHttp.get<Entity[]>(`/api/render_dependence/list`, { key })
}

export function save(entity: Entity) {
  if (entity.id) {
    return requestHttp.put<Entity>(`/api/render_dependence/${entity.id}`, entity)
  }
  return requestHttp.post<Entity>(`/api/render_dependence`, entity)
}

export function remove(id: string) {
  return requestHttp.delete(`/api/render_dependence/${id}`)
}

export function download_to_local(id: string) {
  return requestHttp.get(`/api/render_dependence/download_to_local/${id}`)
}
