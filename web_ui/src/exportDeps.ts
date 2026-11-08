import * as Vue from 'vue'
import * as ElementPlus from 'element-plus'
import * as ElementPlusIcons from '@element-plus/icons-vue'

interface StreamindRuntime {
  sendEvent: (message: string) => PromiseLike<void>
}

const dependence = {
  ElementPlusIcons,
  ElementPlus,
  Vue,
}
;(window as any)['SMD'] = dependence
