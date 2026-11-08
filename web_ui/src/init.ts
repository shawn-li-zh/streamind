export async function initImportMap() {
  try {
    // 1. 从远程获取 map
    const response = await fetch('/public/component-manifest.json')
    const map = await response.json()

    // 2. 创建并注入 script 标签
    const script = document.createElement('script')
    script.type = 'importmap'
    script.textContent = JSON.stringify(map)
    // 确保它被插入到 head 中
    document.head.appendChild(script)
    // 3. 此时 importmap 已生效，可以开始加载 module
    const mainScript = document.createElement('script')
    mainScript.type = 'module'
    mainScript.src = './main.js'
    document.head.appendChild(mainScript)
  } catch (err) {
    console.error('Failed to load importmap:', err)
  }
}
