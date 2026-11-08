import Think from '@/views/chat/section/Think.vue'
import Push from '@/views/chat/section/stack/Push.vue'
import Pop from '@/views/chat/section/stack/Pop.vue'
import Reset from '@/views/chat/section/stack/Reset.vue'
import Reply from '@/views/chat/section/Reply.vue'
import Render from '@/views/chat/section/Render.vue'
import ToolCall from '@/views/chat/section/ToolCall.vue'
import Interrupt from '@/views/chat/section/Interrupt.vue'
import { EventTypeEnum } from '@/api/modules/enums.ts'
import AgentResponse from '@/views/chat/event/AgentResponse.vue'
import UserRequest from '@/views/chat/event/UserRequest.vue'
import SystemError from '@/views/chat/event/SystemError.vue'
import ToolCallResult from '@/views/chat/event/toolCall/ToolCallResult.vue'
import { section } from '@/api/modules'

export type SectionNames = section.Entity['name']
export const sectionComponentMap: Partial<Record<SectionNames, any>> = {
  think: Think,
  reply: Reply,
  render: Render,
  tool_call: ToolCall,
  interrupt: Interrupt,
  push: Push,
  reset: Reset,
  pop: Pop,
}

export const eventComponentMap: Partial<Record<EventTypeEnum, any>> = {
  [EventTypeEnum.AgentResponse]: AgentResponse,
  [EventTypeEnum.UserRequest]: UserRequest,
  [EventTypeEnum.Error]: SystemError,
  [EventTypeEnum.ToolResponse]: ToolCallResult,
}
