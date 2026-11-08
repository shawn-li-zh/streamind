import requestHttp from '@/api'

export function login(code: string) {
  return requestHttp.get<string>('/api/login', {
    code,
  })
}
