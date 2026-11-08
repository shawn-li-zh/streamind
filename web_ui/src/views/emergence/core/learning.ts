// learning/LearningStrategy.ts

import type { Experience } from './domain.ts'

import { VectorN } from './VectorN'

export interface LearningStrategy {
  next_action(exp: Experience): VectorN

  lr: number
}

export class HistoryBasedStrategy implements LearningStrategy {
  private exps: Experience[] = []

  constructor(
    public lr:number=0.1 ,
    private length = 5,
    private explore = 0.2,

  ) {}

  next_action(exp: Experience) {
    this.exps.push(exp)
    if (this.exps.length > this.length) {
      this.exps.shift()
    }
    const dimensions = exp.state.dim()
    const experiences = this.exps
    // ------------------------------------------------
    // 经验不足
    // ------------------------------------------------

    if (experiences.length < 2) {
      return VectorN.random(dimensions, -1, 1).normalize()
    }

    // ------------------------------------------------
    // 随机探索
    // ------------------------------------------------

    if (Math.random() < this.explore) {
      return VectorN.random(dimensions, -1, 1).normalize()
    }

    // ------------------------------------------------
    // 累积奖励梯度
    // ------------------------------------------------

    let preference = VectorN.zero(dimensions)

    for (let i = 1; i < experiences.length; i++) {
      const prev = experiences[i - 1]!
      const curr = experiences[i]!

      const move = curr.state.sub(prev.state)

      const moveLength = move.norm()

      if (moveLength < 1e-6) {
        continue
      }

      const direction = move.normalize()

      const rewardDelta = curr.reward - prev.reward

      /**
       * 奖励变化作为权重
       *
       * rewardDelta > 0
       * 说明这个方向有效
       *
       * rewardDelta < 0
       * 说明这个方向有害
       */
      preference = preference.add(direction.scale(rewardDelta))
    }

    // ------------------------------------------------
    // 没学到东西
    // ------------------------------------------------

    if (preference.norm() < 1e-6) {
      const last = experiences[experiences.length - 1]!

      const prev = experiences[experiences.length - 2]!

      return last.state.sub(prev.state).normalize()
    }

    // ------------------------------------------------
    // 输出偏好方向
    // ------------------------------------------------

    return preference.normalize()
  }
}

export class BestExperienceStrategy {
  private best: Experience | null = null

  constructor(
    public lr:number=0.1 ,
    private explore = 0.1,
    private noise = 0.05,
  ) {}

  next_action(exp: Experience) {
    // ------------------------------------------------
    // 更新最优经验（只存一个）
    // ------------------------------------------------
    if ((!this.best || exp.reward > this.best.reward) && exp.reward != 0) {
      this.best = exp
    }

    const dimensions = exp.state.dim()

    // ------------------------------------------------
    // 探索
    // ------------------------------------------------
    if (Math.random() < this.explore) {
      return VectorN.random(dimensions, -1, 1).normalize()
    }

    // ------------------------------------------------
    // 没有 best
    // ------------------------------------------------
    if (!this.best) {
      return VectorN.random(dimensions, -1, 1).normalize()
    }

    // ------------------------------------------------
    // 利用 best experience
    // ------------------------------------------------
    const dir = this.best.action.clone()

    const action = dir.add(VectorN.random(dimensions, -1, 1).scale(this.noise))

    return action.normalize()
  }
}

export class VelocityStrategy {
  private last: Experience | null = null

  // 当前“惯性方向”
  private velocity: VectorN | null = null

  // 动态步长（关键）
  private stepSize = 0.1

  constructor(
    public lr:number=0.1 ,
    private explore = 0.05,
    private noise = 0.2,
    private minStep = 0.01,
    private maxStep = 1.0,
  ) {}

  next_action(exp: Experience) {
    const dims = exp.state.dim()

    // ------------------------------------------------
    // 初始化
    // ------------------------------------------------
    if (!this.last) {
      this.last = exp
      this.velocity = VectorN.random(dims, -1, 1)
      return this.velocity.clone().normalize().scale(this.stepSize)
    }

    // ------------------------------------------------
    // reward delta（核心信号）
    // ------------------------------------------------
    const delta = exp.reward - this.last.reward

    // 更新 last
    this.last = exp

    // ------------------------------------------------
    // 初始化 velocity
    // ------------------------------------------------
    if (!this.velocity) {
      this.velocity = VectorN.random(dims, -1, 1)
    }

    // ------------------------------------------------
    // exploration（打破局部吸引）
    // ------------------------------------------------
    if (Math.random() < this.explore) {
      const rand = VectorN.random(dims, -1, 1)
      this.velocity = this.velocity.scale(0.8).add(rand.scale(this.noise))

      return this.velocity.clone().normalize().scale(this.stepSize)
    }

    // ------------------------------------------------
    // reward improvement → 强化惯性
    // reward下降 → 扰动
    // ------------------------------------------------
    if (delta > 0) {
      // 趋势正确：增强惯性 + 增大步长
      this.velocity = this.velocity.scale(0.9).add(this.velocity.clone().scale(0.3))
      this.stepSize = Math.min(this.maxStep, this.stepSize * 1.05)
    } else {
      // 趋势错误：打散 + 缩小步长
      const noiseVec = VectorN.random(dims, -1, 1)
      this.velocity = this.velocity.scale(0.6).add(noiseVec.scale(this.noise))
      this.stepSize = Math.max(this.minStep, this.stepSize * 0.95)
    }

    // ------------------------------------------------
    // 输出 action（关键：不再纯 normalize 掉信息）
    // ------------------------------------------------
    const action = this.velocity.clone().normalize().scale(this.stepSize)

    return action
  }
}
