export interface StreamindRuntime {
    //发送ui交互事件
    sendEvent: (message: string) => Promise<void>
    getLocalImageUrl: (path: string) => string
}
