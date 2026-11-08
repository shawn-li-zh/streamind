const CONFIG = {
  MAX_SIBLINGS: 3, // 每种tag最多保留的相同子节点数量
  FOLD_THRESHOLD: 100, // end - start 大于此值时，开启同级节点折叠
}

const none_readable_elements = ['script', 'style', 'map', 'meta', 'link', 'noscript', 'br', 'hr']
// svg 被认定为叶子节点，不再深度解析其内部复杂的绘制节点
const leaf_tags = ['input', 'pre', 'img', 'canvas', 'iframe', 'video', 'svg', 'textarea']

interface Node {
  id: number
  maxId?: number // 记录该节点所包含子树中的最大 ID (用于快速跳过不在区间的子树)
  tag: string
  attrs?: Record<string, string | number | boolean | null | undefined>
  role?: string
  clickable?: boolean
  width?: number
  height?: number
  scrollX?: boolean
  scrollY?: boolean
  children: Node[]
  text?: string
  layout?: string
  element: HTMLElement | null
  isEllipsis?: boolean
  depth?: number
}

const KEY = 'STREAMIND'
const ref = {
  eleMap: new Map<number, Node>(),
  root: null as Node | null,
}

// ======================= 核心解析逻辑 =======================

function clickable(ele: HTMLElement, style: CSSStyleDeclaration) {
  if (!ele) return false
  if (ele.tagName === 'A' || ele.tagName === 'BUTTON') return true
  if (style.cursor === 'pointer') return true
  const role = ele.getAttribute('role')
  if (role && ['button', 'checkbox', 'tab', 'menuitem', 'link'].includes(role)) return true
  return false
}

function getLayout(parent: HTMLElement, style: CSSStyleDeclaration): string | undefined {
  const display = style.display
  if (display.includes('flex')) {
    return style.flexDirection.includes('column') ? 'L=V' : 'L=H'
  } else if (display.includes('grid')) {
    return 'L=G'
  }
  return undefined
}

function buildRawTree(ele: HTMLElement): Node | undefined {
  if (!ele) return
  const tagName = ele.tagName.toLowerCase()
  if (none_readable_elements.includes(tagName)) return

  const style = window.getComputedStyle(ele)
  // display: none 会使所有子节点也强行从渲染树抹除，直接跳过
  if (style.display === 'none' || parseFloat(style.opacity) === 0) return

  const rect = ele.getBoundingClientRect()
  const isSelfVisible = style.visibility !== 'hidden' && rect.width > 0 && rect.height > 0

  const node: Node = {
    id: 0,
    tag: tagName,
    clickable: clickable(ele, style),
    role: ele.getAttribute('role') || undefined,
    width: rect.width,
    height: rect.height,
    scrollX: style.overflowX === 'auto' || style.overflowX === 'scroll',
    scrollY: style.overflowY === 'auto' || style.overflowY === 'scroll',
    children: [],
    element: ele,
    layout: getLayout(ele, style),
  }

  // 遇到 svg 等 leaf_tags 直接停止递归子元素，只占位
  if (!leaf_tags.includes(tagName)) {
    const children = Array.from(ele.children)
    for (const child of children) {
      const subNode = buildRawTree(child as HTMLElement)
      if (subNode) {
        node.children.push(subNode)
      }
    }
  }

  // 修复：如果父节点自身不可见，但只要有可见的子元素被提取到了（比如子元素用了绝对定位/覆盖），
  // 就必须保留这个父元素作为容器，否则会破坏结构。没有任何可见子元素才抛弃。
  if (!isSelfVisible && node.children.length === 0) {
    return
  }

  if (isSelfVisible) {
    if (tagName === 'input' || tagName === 'textarea') {
      node.attrs = {
        type: ele.getAttribute('type'),
        placeholder: ele.getAttribute('placeholder'),
      }
      node.text = (ele as HTMLInputElement).value || ''
    } else if (tagName === 'img') {
      node.text = ele.getAttribute('alt') || ''
    } else if (leaf_tags.includes(tagName)) {
      const inner = ele.innerText
      if (inner) node.text = inner.trim().replace(/\s+/g, ' ')
    } else {
      let directText = ''
      for (const child of ele.childNodes) {
        if (child.nodeType === window.Node.TEXT_NODE) {
          directText += child.textContent || ''
        }
      }
      const cleanText = directText.trim().replace(/\s+/g, ' ')
      if (cleanText) node.text = cleanText
    }
  }

  return node
}

// ======================= 树压缩与优化 =======================

function collapseTree(node: Node): Node {
  node.children = node.children.map(collapseTree).filter(Boolean)

  if (node.children.length === 1 && !node.text && !leaf_tags.includes(node.tag)) {
    const child = node.children[0]!
    return {
      ...child,
      tag: child.tag,
      element: child.element,
      layout: node.layout || child.layout,
      width: node.width || child.width,
      height: node.height || child.height,
      scrollX: node.scrollX || child.scrollX,
      scrollY: node.scrollY || child.scrollY,
      clickable: node.clickable || child.clickable,
      role: child.role || node.role,
      attrs: { ...(node.attrs || {}), ...(child.attrs || {}) },
    }
  }

  return node
}

function assignIdsAndCache(node: Node, idCounter: { current: number }, depth: number = 0) {
  node.id = idCounter.current++
  node.depth = depth
  ref.eleMap.set(node.id, node)
  if (node.element) {
    node.element.setAttribute('streamind-id', node.id.toString())
  }
  for (const child of node.children) {
    assignIdsAndCache(child, idCounter, depth + 1)
  }
  // 记录该子树下的最大ID，用于后续精准快速跳过不在区间的树枝
  node.maxId = idCounter.current - 1
}

function ellipsis(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text
  const midLength = Math.floor(maxLength / 2)
  return text.substring(0, midLength) + '...' + text.substring(text.length - midLength)
}

// ======================= 输出构建 =======================

function buildNodeString(node: Node, level: number): string {
  if (node.isEllipsis) {
    return '\t'.repeat(level) + `... <${node.tag}> * (Auto Omitted)`
  }

  const items: string[] = []
  if (node.layout) items.push(node.layout)
  if (node.width && node.height)
    items.push(`S=${Math.round(node.width)}x${Math.round(node.height)}`)

  if (node.scrollX && node.scrollY) items.push(`scroll="X&Y"`)
  else if (node.scrollX) items.push(`scroll="X"`)
  else if (node.scrollY) items.push(`scroll="Y"`)

  if (node.clickable) items.push('clickable')
  if (node.role) items.push(`role='${node.role}'`)

  if (node.attrs) {
    Object.entries(node.attrs).forEach(([k, v]) => {
      if (v) items.push(`${k}="${v}"`)
    })
  }

  const attrs = items.join(' ')
  let head = '\t'.repeat(level) + `[${node.id}] <${node.tag}${attrs ? ' ' + attrs : ''}>`

  if (node.text) {
    head += `:"${ellipsis(node.text, 30)}"`
  }
  return head
}

/**
 * 查找并格式化输出节点树
 * @param start 起始节点ID (0 为从 document.body 开始)
 * @param end 结束节点ID (可选)
 */
function look_dom(start: number = 0, end?: number): string {
  if (!ref.root) {
    return 'Dom node tree not initialized.'
  }
  const actualStart = start === 0 ? 1 : start
  const maxExistingId = ref.eleMap.size
  const actualEnd = end !== undefined && end !== -1 ? end : maxExistingId
  const safeEnd = Math.min(actualEnd, maxExistingId)

  // 1. 扫描当前区间，获取“最浅层级（最顶层的那个祖先节点深度）”作为相对缩进的 0 基准
  let minDepth = Infinity
  for (let i = actualStart; i <= safeEnd; i++) {
    const node = ref.eleMap.get(i)
    if (node && node.depth !== undefined) {
      minDepth = Math.min(minDepth, node.depth)
    }
  }
  const baseDepth = minDepth === Infinity ? 0 : minDepth

  const lines: string[] = []
  const enableFolding = actualEnd - actualStart > CONFIG.FOLD_THRESHOLD

  // 2. 无折叠全量输出：直接顺序遍历 ID，天然具备结构保真的 DFS 顺序
  if (!enableFolding) {
    for (let i = actualStart; i <= safeEnd; i++) {
      const node = ref.eleMap.get(i)
      if (node) {
        const printLevel = Math.max(0, node.depth! - baseDepth)
        lines.push(buildNodeString(node, printLevel))
      }
    }
    return lines.join('\n')
  }

  // 3. 超出阈值折叠模式：通过 DFS 并利用 maxId 进行快速剪枝
  let reachedEnd = false

  function traverse(node: Node) {
    if (reachedEnd) return
    if (node.id > actualEnd) {
      reachedEnd = true
      return
    }
    // 性能优化：如果该节点包含的所有子节点最大 ID 都比起始 ID 小，这棵树完全不需要进去看
    if (node.maxId !== undefined && node.maxId < actualStart) {
      return
    }

    // 在区间内，进行输出
    if (node.id >= actualStart && node.id <= actualEnd) {
      const printLevel = Math.max(0, node.depth! - baseDepth)
      lines.push(buildNodeString(node, printLevel))
    }

    if (!node.children || node.children.length === 0) return

    const groupedChildren: Node[] = []
    const tagCountMap = new Map<string, number>()

    for (const child of node.children) {
      const count = (tagCountMap.get(child.tag) || 0) + 1
      tagCountMap.set(child.tag, count)

      if (count <= CONFIG.MAX_SIBLINGS) {
        groupedChildren.push(child)
      } else if (count === CONFIG.MAX_SIBLINGS + 1) {
        groupedChildren.push({
          id: child.id,
          maxId: child.maxId,
          tag: child.tag,
          children: [],
          element: null,
          isEllipsis: true,
          depth: child.depth,
        })
      }
    }

    for (const child of groupedChildren) {
      if (reachedEnd) break
      if (child.isEllipsis) {
        if (child.id > actualEnd) {
          reachedEnd = true
          break
        }
        if (child.maxId !== undefined && child.maxId < actualStart) continue

        if (child.id >= actualStart) {
          const printLevel = Math.max(0, child.depth! - baseDepth)
          lines.push(buildNodeString(child, printLevel))
        }
      } else {
        traverse(child)
      }
    }
  }

  traverse(ref.root)
  return lines.join('\n')
}

function resolve_dom(ele?: HTMLElement) {
  const rawTree = buildRawTree(ele || document.body)
  if (rawTree) {
    const counter = { current: 0 }
    const collapsedTree = collapseTree(rawTree)
    assignIdsAndCache(collapsedTree, counter, 0)
    ref.root = collapsedTree
    return `node range[0,${counter.current}]`
  }
  return 'Failed to parse DOM.'
}

// ======================= 初始化与挂载 =======================

function init() {
  console.log('[STREAMIND] Start parsing DOM...')
  resolve_dom()
  ;(window as any)[KEY] = ref
  ;(window as any)['look_dom'] = look_dom
  ;(window as any)['resolve_dom'] = resolve_dom
  ;(window as any)['node_element'] = (id: number) => ref.eleMap.get(id)?.element

  console.log('[STREAMIND] Inject ready.')
}

init()
