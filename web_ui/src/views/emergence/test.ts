import { VectorN } from '@/views/emergence/core/VectorN.ts'
// import { Entity, Race, World, RaceTrait } from './domain/'
import { VelocityStrategy } from '@/views/emergence/core/learning.ts'
import type { RaceTrait } from '@/views/emergence/core/domain.ts'
import { Entity, Race, World } from '@/views/emergence/core/domain.ts'

/**
 * 扩展 World 以包含时间滴答
 */
export class TemporalWorld extends World {
  public tick: number = 0
  public history: number[] = [] // 记录 L1 全体激活度的历史，用于 R2 评估节奏

  step_tick() {
    this.tick++
  }
}

/**
 * R1 种群：基础脉冲单元
 * 状态 [0]: 当前激活度 (0~1)
 * 状态 [1]: 疲劳累积 (激活时间越长，疲劳越高)
 */
export const r1PulseRace: RaceTrait<any> = {
  default_state(): VectorN {
    return new VectorN([0, 0]) // 初始不激活，疲劳为0
  },

  update(entity: Entity<any>, state: VectorN, action: VectorN): VectorN {
    let activation = state.values[0]! + action.values[0]!
    activation = Math.max(0, Math.min(1, activation))

    let fatigue = state.values[1]!
    if (activation > 0.5) {
      fatigue += 0.05 // 持续激活增加疲劳
    } else {
      fatigue -= 0.1 // 休息降低疲劳
    }
    fatigue = Math.max(0, Math.min(2, fatigue))

    return new VectorN([activation, fatigue])
  },

  reward(entity: Entity<any>, state: VectorN, world: World): number {
    const activation = state.values[0]!
    const fatigue = state.values[1]!

    // 内生本能：想激活，但讨厌疲劳
    // 如果不受到外界奖励，它会因为疲劳而被迫进入“休眠-激活”的极简循环
    return activation * 0.5 - (fatigue > 1.0 ? fatigue * 2 : 0)
  },

  rectify_underlying_entity_reward(): number {
    return 0 // 最底层不修正
  },
}

/**
 * R2 种群：节奏协调者
 * 状态: 这是一个 N 维向量，代表一个固定长度时间环里的“能量分配”
 * 比如 10 维，代表 t%10 的时刻分别给多少奖励
 */
export const r2RhythmRace: RaceTrait<any> = {
  default_state(): VectorN {
    // 初始随机生成 16 个时间点的奖励偏置
    return VectorN.random(16, -1, 1)
  },

  update(entity: Entity<any>, state: VectorN, action: VectorN): VectorN {
    // R2 在不断微调它的“节奏盘”参数
    const next = state.add(action)
    return next.map(v => Math.max(-2, Math.min(2, v)))
  },

  reward(entity: Entity<any>, state: VectorN, world: TemporalWorld): number {
    const cells = entity.cells
    if (cells.length === 0) return 0

    // 计算当前时刻全体 R1 的总激活度
    const totalActivation =
      cells.reduce((sum, e) => sum + e.cur_exp.state.values[0]!, 0) / cells.length
    world.history.push(totalActivation)
    if (world.history.length > 32) world.history.shift()

    // --- R2 的目标：产生“节奏感” ---
    // 节奏感定义：激活度随时间的变化率（方差）要大。
    // 如果大家一直半死不活（平滑），得分低；如果像心跳一样忽开忽关，得分高。
    const avg = world.history.reduce((a, b) => a + b, 0) / world.history.length
    const variance = world.history.reduce((s, v) => s + (v - avg) ** 2, 0) / world.history.length

    // 额外奖励：如果总激活度的平均值在 0.3~0.6 之间（代表有节制的循环，而不是全体暴走或全体熄灭）
    const balanceBonus = avg > 0.2 && avg < 0.7 ? 10 : -5

    return variance * 100 + balanceBonus
  },

  /**
   * 核心：时序修正函数
   */
  rectify_underlying_entity_reward(
    this_entity: Entity<any>,
    under_entity: Entity<any>,
    world: TemporalWorld,
  ): number {
    // 获取 R2 学习出来的“节奏盘”
    const rhythmDisk = this_entity.cur_exp.state.values
    const diskSize = rhythmDisk.length

    // 根据当前世界时间，决定现在的奖励增益
    const currentPhase = world.tick % diskSize
    const energyLevel = rhythmDisk[currentPhase]!

    // 下层实体感知到的奖励 = 它自己的激活度 * 当前时间点的能量强度
    const r1Activation = under_entity.cur_exp.state.values[0]!

    // 这里没有 IF，是一个纯粹的场叠加：
    // 如果 R2 在这一时刻（Phase）设置了高能量，那么 R1 只要激活就能拿到高分。
    return r1Activation * energyLevel * 10
  },
}

// --- 初始化与运行示例 ---

const world = new TemporalWorld()
const r1Race = new Race('r1', {}, r1PulseRace, () => new VelocityStrategy(0.1, 0.1))
const r2Race = new Race('r2', {}, r2RhythmRace, () => new VelocityStrategy(0.05, 0.1))

// 1. 创建层级
const root = new Entity(-1, {} as any, null as any, null as any) // 虚构根节点
const conductor = r2Race.new_entity(root) // R2 层
for (let i = 0; i < 20; i++) {
  r1Race.new_entity(conductor) // R1 层
}

// 2. 模拟运行
setInterval(() => {
  world.step_tick()

  // 第一阶段：R2 思考
  const r2Action = conductor.next_action()

  // 第二阶段：R1 思考
  const r1Actions = conductor.cells.map(e => e.next_action())

  // 第三阶段：更新
  conductor.step(r2Action, world)
  conductor.cells.forEach((e, i) => e.step(r1Actions[i], world))

  // 打印当前信号：你会看到它从杂乱无章逐渐变成 [0,0,1,1,0,0,1,1] 这样的循环
  const signal = conductor.cells.map(e => e.cur_exp.state.values[0]!.toFixed(1))
  console.log(`Tick: ${world.tick} | Pattern: ${signal.join('|')}`)
}, 100)
