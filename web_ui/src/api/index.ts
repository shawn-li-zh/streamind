import type { AxiosError, AxiosInstance, AxiosRequestConfig, AxiosResponse } from 'axios'
import axios from 'axios'
// import { tryHideFullScreenLoading } from "@/components/Loading/fullScreen";
import { ElMessage } from 'element-plus'
import type { ResultData } from '@/api/interface'
import { AxiosCanceler } from './helper/axiosCancel'
import router from '@/router'

export interface CustomAxiosRequestConfig extends AxiosRequestConfig {
  loading?: boolean
  cancel?: boolean
  idempotent?: string
  cacheParams?: Record<string, any>
}

const apiUrl = import.meta.env.VITE_API_URL as string
let url = apiUrl
if (!url) {
  url = `/`
}
const config = {
  // 默认地址请求地址，可在 .env.** 文件中修改
  baseURL: url,
  // 设置超时时间
  timeout: 30000,
  // 跨域时候允许携带凭证
  withCredentials: true,
}

const axiosCanceler = new AxiosCanceler()

class RequestHttp {
  service: AxiosInstance
  httpCache: Record<string, Promise<ResultData<any>>>

  public constructor(config: AxiosRequestConfig) {
    // instantiation
    this.service = axios.create(config)
    this.httpCache = {}
    /**
     * @description 请求拦截器
     * 客户端发送请求 -> [请求拦截器] -> 服务器
     * token校验(JWT) : 接受服务器返回的 token,存储到 vuex/pinia/本地储存当中
     */
    this.service.interceptors.request.use(
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      //@ts-expect-error
      configTemp => {
        const config = configTemp as unknown as CustomAxiosRequestConfig
        //将参数中的array类型的参数转为字符串
        if (config.params) {
          Object.keys(config.params).forEach(key => {
            const param = config.params[key]
            if (Array.isArray(param) && param.length > 0) {
              const ty = typeof param[0]
              if (ty == 'string' || ty == 'number' || ty == 'boolean') {
                config.params[key] = param.join(',')
              }
            }
          })
        }

        return config
      },
      (error: AxiosError) => {
        return Promise.reject(error)
      },
    )

    /**
     * @description 响应拦截器
     *  服务器换返回信息 -> [拦截统一处理] -> 客户端JS获取到信息
     */
    this.service.interceptors.response.use(
      (response: AxiosResponse & { config: CustomAxiosRequestConfig }) => {
        const { config } = response
        const result: ResultData = response.data
        axiosCanceler.removePending(config)
        // config.loading && tryHideFullScreenLoading();
        // 全局错误信息拦截（防止下载文件的时候返回数据流，没有 code 直接报错）
        if (result instanceof Blob) {
          return result
        }
        if (result.code && result.code !== 200) {
          ElMessage.error(result.message)
          return Promise.reject(result)
        }
        // 成功请求（在页面上除非特殊情况，否则不用处理失败逻辑）
        return result as any
      },
      async (error: AxiosError) => {
        const { response } = error
        // tryHideFullScreenLoading();
        // 请求超时 && 网络错误单独判断，没有 response
        if (error.message.indexOf('timeout') !== -1) ElMessage.error('请求超时！请您稍后重试')
        else if (error.message.indexOf('Network Error') !== -1)
          ElMessage.error('网络错误！请您稍后重试')
        // 根据服务器响应的错误状态码，做不同的处理
        else if (response && response.data) {
          const data: any = response.data
          if (data.message) {
            ElMessage.error(data.message)
          }
          if (data.code == 401) {
            //
            await router.replace('/login')
          }
          return Promise.reject(error)
        }
        ElMessage.error(response?.statusText || error.message)
        // 服务器结果都没有返回(可能服务器错误可能客户端断网)，断网处理:可以跳转到断网页面
        // if (!window.navigator.onLine) await router.replace('/500')
        return Promise.reject(error)
      },
    )
  }

  /**
   * @description 常用请求方法封装
   */
  get<T>(url: string, params?: object, config?: CustomAxiosRequestConfig): Promise<ResultData<T>> {
    return this.service.get(url, { params, ...config })
  }

  post<T>(
    url: string,
    data?: object | string,
    config?: CustomAxiosRequestConfig,
  ): Promise<ResultData<T>> {
    return this.service.post(url, data, config)
  }

  put<T>(url: string, data?: object, config?: CustomAxiosRequestConfig): Promise<ResultData<T>> {
    return this.service.put(url, data, config)
  }

  delete<T>(url: string, params?: any, config?: CustomAxiosRequestConfig): Promise<ResultData<T>> {
    return this.service.delete(url, { params, ...config })
  }

  download(url: string, params?: object, config?: CustomAxiosRequestConfig): Promise<BlobPart> {
    return this.service.request({ ...config, url, params, responseType: 'blob' })
  }

  async downloadImmediately(
    url: string,
    params?: object,
    config?: CustomAxiosRequestConfig,
  ): Promise<void> {
    let headers: any = {}
    const blobPart = await this.download(url, params, {
      ...(config || {}),
      transformResponse: (data, head) => {
        headers = head
        return data
      },
    })
    const blob = new Blob([blobPart])
    const name = (headers['content-disposition'] || '').match(/filename=(.*)/)?.[1] || ''
    const filename = decodeURI(name)
    // 兼容 edge 不支持 createObjectURL 方法
    if ('msSaveOrOpenBlob' in navigator)
      return (window.navigator as any).msSaveOrOpenBlob(blob, filename)
    const blobUrl = window.URL.createObjectURL(blob)
    const exportFile = document.createElement('a')
    exportFile.style.display = 'none'
    exportFile.download = filename
    exportFile.href = blobUrl
    document.body.appendChild(exportFile)
    exportFile.click()
    // 去除下载对 url 的影响
    document.body.removeChild(exportFile)
    window.URL.revokeObjectURL(blobUrl)
  }

  //返回完整地址
  getUri(url: string, params?: object, config?: object): string {
    return this.service.getUri({ url, params, ...config })
  }

  /**
   * 缓存get请求
   * @param url
   * @param params
   * @param config
   */
  async cacheGet<T>(
    url: string,
    params?: object,
    config?: CustomAxiosRequestConfig,
  ): Promise<ResultData<T>> {
    const cacheKey = this.getUri(url, config?.cacheParams || params, {})
    let res = this.httpCache[cacheKey]
    if (res) {
      return await res
    }
    res = this.get<T>(url, params, { ...config, method: 'get' })
    this.httpCache[cacheKey] = res
    const result = await res
    if (!result.isOk) {
      delete this.httpCache[cacheKey]
    }
    return result
  }

  async sseConnect(
    url: string,
    callback: (data: string) => PromiseLike<void>,
    options?: {
      params?: object
      abortController?: AbortController
      onConnect?: () => void
      heartbeatTimeout?: number // 心跳超时时间
    },
  ) {
    const uri = this.getUri(url, options?.params || {})
    const timeoutMs = options?.heartbeatTimeout || 1000*10
    let watchdogTimer: any = null

    // 辅助函数：重置看门狗定时器
    const resetWatchdog = () => {
      if (watchdogTimer) clearTimeout(watchdogTimer)
      watchdogTimer = setTimeout(() => {
        console.warn('SSE 心跳超时，主动断开连接')
        options?.abortController?.abort() // 触发 abort，从而引发 fetch 异常进入 catch
      }, timeoutMs)
    }

    // 1. 发起请求
    const response = await fetch(uri, {
      method: 'GET',
      headers: { Accept: 'text/event-stream' },
      signal: options?.abortController?.signal,
    })

    // 2. 检查 HTTP 状态码
    // 如果后台返回 5xx 或 4xx，fetch 不报错，但这里必须手动抛错触发重连
    if (!response.ok) {
      const errorText = await response.text().catch(() => 'Unknown Error')
      throw new Error(`HTTP Error: ${response.status} - ${errorText}`)
    }

    if (!response.body) throw new Error('No response body')

    if (options?.onConnect) options.onConnect()

    const reader = response.body.getReader()
    const decoder = new TextDecoder('utf-8')
    let buffer = ''

    try {
      while (true) {
        if (!response.ok) break
        const { value, done } = await reader.read()
        // 每次有数据过来，重置计时器
        resetWatchdog()
        // 3. 当后台关闭连接或流结束
        if (done) {
          console.warn('SSE Reader done (stream closed by server)')
          break
        }

        buffer += decoder.decode(value, { stream: true })
        const parts = buffer.split('\n\n')
        buffer = parts.pop() || ''

        for (const part of parts) {
          if (!part.startsWith('data:')) continue
          const dataBody = part.replace(/^data:\s*/, '')
          if (!dataBody || dataBody === '[DONE]') continue
          await callback(dataBody)
        }
      }
    } catch (e: any) {
      // 捕获读取流时的异常（如网络突然中断）
      if (e.name === 'AbortError') {
        console.log('Connection aborted by user')
        return // 主动取消，不触发外层重连
      }
      throw e // 抛给外层触发重连
    } finally {
      reader.releaseLock()
    }

    // 4. 重点：跳出 while 循环意味着连接断开了，必须抛出一个错误让外层 catch 到
    throw new Error('SSE connection closed by server')
  }
}

const requestHttp = new RequestHttp(config)
export default requestHttp
