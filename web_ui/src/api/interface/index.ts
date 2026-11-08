// 请求响应参数（不包含data）

export interface Result {
  code: number | string
  message: string
  timestamp: number
  isOk: boolean
  extensions: Record<string, any>
}

// 请求响应参数（包含data）
export interface ResultData<T = any> extends Result {
  data: T
}

// 分页响应参数
export interface ResultPage<T> {
  records: T[]
  current: number
  pageSize: number
  total: number | string
}

// 区间参数
export interface RangeParameter<T> {
  start?: T
  end?: T
}

// 分页请求参数
export interface PageParameter {
  pageIndex: number
  pageSize: number
}

export type SortParameter<T = any> = {
  [P in keyof T]?: 'asc' | 'desc'
}

export function sortToString(sort?: SortParameter) {
  if (!sort) return ''
  return Object.keys(sort)
    .map((key) => `${key} ${sort[key]}`)
    .join(',')
}

export interface CommonEntity {
  isDeleted?: boolean
  createTime: string
  createBy: string
  updateTime?: string
  updateBy?: string
}

export interface StrictEntity extends CommonEntity {
  isDeleted: boolean
  deleteTime?: string
  deleteBy?: string
}
