interface ToolConfig {
  logo: string
  title: string
  explain: (
    args: Record<string, any>,
    callInfo?: {
      package: string
      tool: string
    },
  ) => string
}

const toolMap: Record<string, Record<string, ToolConfig>> = {
  tool: {
    default: {
      logo: 'tool',
      title: '元工具管理',
      explain: (
        args: Record<string, any>,
        callInfo?: {
          package: string
          tool: string
        },
      ) => {
        const map: Record<string, string> = {
          list_packages: `搜索工具包`,
          list_tools: `搜索包内工具:${args.pkg_name}`,
          get_tool_info: `查看工具完整定义:${args.pkg_name}.${args.tool_name}`,
          get_type_info: `查看类型完整定义:${args.pkg_name}.${args.type_name}`,
        }
        return `${map[callInfo!.tool] || ''}`
      },
    },
  },
  http: {
    default: {
      logo: 'http',
      title: 'http网络请求',
      explain: (
        args: Record<string, any>,
        callInfo?: {
          package: string
          tool: string
        },
      ) => {
        const map: Record<string, string> = {
          show_config: `查看配置`,
          update_config: `更新配置`,
        }
        return `${map[callInfo!.tool] || ''}`
      },
    },
    request: {
      logo: 'http',
      title: 'http网络请求',
      explain: (args: Record<string, any>) => `${args.method.toString().toUpperCase()} ${args.url}`,
    },
    detail: {
      logo: 'http',
      title: '查看请求详情',
      explain: (args: Record<string, any>) => `查看请求详情`,
    },
  },
  event: {
    default: {
      logo: 'tool',
      title: '历史事件检索',
      explain: (
        args: Record<string, any>,
        callInfo?: {
          package: string
          tool: string
        },
      ) => {
        const map: Record<string, string> = {
          get_by_id: `查询ID${args.id}对应的事件`,
          search_at: `搜索指定事件发生${(args.before ?? true) ? '之前' : '之后'}的事件列表`,
          search: `搜索时间范围内的事件列表:${args.start}...${args.end} 关键字:${args.key || '无'}`,
        }
        return `${map[callInfo!.tool] || ''}`
      },
    },
  },
  editor: {
    load_file: {
      logo: 'glasses',
      title: '加载文件',
      explain: (args: Record<string, any>) => `加载:${args.path}`,
    },
    read: {
      logo: 'glasses',
      title: '文件读取',
      explain: (args: Record<string, any>) => `阅读文件:${args.pos}...${args.len}`,
    },
    write: {
      logo: 'write',
      title: '文件写入',
      explain: (args: Record<string, any>) => `写入文件`,
    },
  },
  timer: {
    sleep: {
      logo: 'sleep',
      title: '休眠',
      explain: (args: Record<string, any>) => `休眠:${args.time}`,
    },
  },
  browser: {
    default: {
      logo: 'tool',
      title: '浏览器工具',
      explain: (
        args: Record<string, any>,
        callInfo?: {
          package: string
          tool: string
        },
      ) => {
        const map: Record<string, string> = {
          screenshot: '浏览器网页截图',
          pipeline: '指令流水线',
          look_dom: '查看网页',
          look_iframes: '查看frame树',
          resolve_dom: '分析网页',
          exec: '执行js脚本',
          list_tabs: '获取标签页列表',
          close_tab: '关闭标签页',
          switch_tab: '切换标签页',
          new_tab: `新建标签页:${args.url}`,
          close: '关闭浏览器',
          launch: '尝试启动浏览器',
        }
        return `${map[callInfo!.tool] || ''}`
      },
    },
  },
  reflect: {
    default: {
      logo: 'alert',
      title: '记忆管理',
      explain: (
        args: Record<string, any>,
        callInfo?: {
          package: string
          tool: string
        },
      ) => {
        const map: Record<string, string> = {
          page_groups: `分页查询反射组的基础信息`,
          page_rules: `分页查询组内的反射规则`,
          page_concepts: `分页查询已定义的概念`,
          new_group: `创建新分组：${args.name}`,
          new_concept: `定义名词概念：${args.name}`,
          save_rule: `保存反射规则：${args.name}`,
          current_groups: `查看已经加载的规则组`,
          load_groups: `加载规则组：${(args.group_names||[]).join(', ')}`,
          unload_groups: `卸载规则组：${(args.group_names||[]).join(', ')}`,
        }
        return `${map[callInfo!.tool] || ''}`
      },
    },
  },
}

export function getToolConfig(packageName: string, toolName: string) {
  let originalConfig = toolMap[packageName]?.[toolName]
  if (!originalConfig) {
    originalConfig = toolMap[packageName]?.['default']
  }

  // 如果还是没有，给个默认值
  if (!originalConfig) {
    return {
      logo: 'tool',
      title: '未知工具',
      explain: (args: Record<string, any>) => `调用:${packageName || ''}.${toolName || ''}`,
    }
  }

  // ✅ 关键点：使用扩展运算符 {...} 创建一个新对象，避免修改原始 toolMap
  const config = { ...originalConfig }

  const raw = config.explain
  const callInfo = {
    package: packageName,
    tool: toolName,
  }

  // 这里修改的是新对象的 explain，不会影响 toolMap 里的 default
  config.explain = (args: Record<string, any>) => {
    return raw(args, callInfo)
  }

  return config
}
