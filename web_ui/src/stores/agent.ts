import { defineStore } from 'pinia'
import * as module from '@/api/modules'
import { agent, event } from '@/api/modules'
import piniaPersistConfig from '@/stores/helper/persist.ts'
import { EventTypeEnum } from '@/api/modules/enums.ts'

const id = 'agent'

export interface EventAndSections {
  event: module.event.Entity
  sections: module.section.Entity[]
}

export interface AgentInfo {
  agent_id: string
  connected: boolean
  inited: boolean
  step: number
  data: agent.Entity
  reactions: EventAndSections[]
  messages: EventAndSections[]
  last_response_event: {
    reaction?: EventAndSections
    message?: EventAndSections
  }
  state_stack: module.state.Entity[]
}

export interface AgentPool {
  agents: Record<string, AgentInfo>
  agent_list: agent.Entity[]
  last_agent_id: string
  inited: boolean
}

export const useAgentStore = defineStore(id, {
  state: (): AgentPool => ({
    agents: {},
    agent_list: [],
    last_agent_id: '',
    inited: false,
  }),
  getters: {},
  actions: {
    getAgent(id: string): AgentInfo {
      let agent = this.agents[id]
      if (agent) {
        if (!agent.inited) {
          initAgent(agent)
        }
        return agent
      }
      //
      this.agents[id] = wrapAgent({ id: id } as any)
      agent = this.agents[id]
      this.last_agent_id = id
      initAgent(agent)
      return agent
    },
    async loadMoreEvents(id: string): Promise<boolean> {
      const agent = this.agents[id]
      if (!agent) return false
      return await loadAgentEvents(agent)
    },
    async init() {
      if (this.inited) return
      const { data } = await module.agent.tree()
      this.agent_list = data
      data.forEach(agent => {
        this.agents[agent.id] = wrapAgent(agent)
        agent.children?.forEach(child => {
          this.agents[child.id] = wrapAgent(child)
        })
      })
      if (!this.agents[this.last_agent_id]) {
        this.last_agent_id = data[0]?.id || ''
      }
      this.inited = true
    },
  },

  persist: piniaPersistConfig(id, ['last_agent_id']),
})

async function fillAgentInfo(info: AgentInfo) {
  const { data } = await agent.get_by_id(info.agent_id)
  info.data = data
  //
  await loadAgentEvents(info)
}

function initAgent(info: AgentInfo) {
  info.inited = true
  Promise.all([fillAgentInfo(info), loadAgentState(info)])
    .then(() => {
      console.log('agent info loaded', info)
    })
    .catch(e => {
      console.log('agent info load error', e)
      info.connected = false
      info.inited = false
    })
}

function wrapAgent(agentEntity: agent.Entity): AgentInfo {
  return {
    agent_id: agentEntity.id,
    connected: false,
    inited: false,
    step: 0,
    data: agentEntity,
    last_response_event: {},
    reactions: [],
    messages: [],
    state_stack: [],
  }
}

async function loadAgentEvents(info: AgentInfo): Promise<boolean> {
  const first = info.messages[0]
  const size=50
  const { data: events } = await event.load(info.agent_id, size, first?.event.id, [
    EventTypeEnum.UserRequest,
    EventTypeEnum.Interrupt,
    EventTypeEnum.AgentResponse,
  ])
  const start_event_id = events[0]?.id || '0'
  const end_event_id = events.at(-1)?.id || '0'
  const { data: event_and_sections } = await module.section.list_in_range(
    info.agent_id,
    start_event_id,
    end_event_id,
  )
  //
  const reactions: EventAndSections[] = []
  const messages: EventAndSections[] = []
  for (const obj of event_and_sections) {
    switch (obj.event.event_type) {
      case EventTypeEnum.AgentResponse:
        const r: module.section.Entity[] = []
        const m: module.section.Entity[] = []
        for (const sec of obj.sections) {
          switch (sec.name) {
            case 'render':
            case 'reply':
              m.push(sec)
              break
            default:
              r.push(sec)
          }
        }
        if (r.length) {
          reactions.push({
            event: obj.event,
            sections: r,
          })
        }
        if (m.length) {
          messages.push({
            event: obj.event,
            sections: m,
          })
        }
        break
      case EventTypeEnum.UserRequest:
      case EventTypeEnum.Interrupt:
        messages.push(obj)
        break
    }
  }
  info.reactions = [...reactions, ...info.reactions]
  info.messages = [...messages, ...info.messages]
  return events.length == size
}

async function loadAgentState(info: AgentInfo) {
  const { data } = await module.state.list(info.agent_id)
  info.state_stack = data
}
