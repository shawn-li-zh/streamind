import { VectorN } from '@/views/emergence/core/VectorN'
import { Entity, Race, World } from '@/views/emergence/core/domain'
import {
  BestExperienceStrategy,
  HistoryBasedStrategy,
  VelocityStrategy,
} from '@/views/emergence/core/learning.ts'

export interface Scope {
  width: number
  height: number
  centerX: number
  centerY: number
  bools: [boolean, boolean]
}

const scope: Scope = {
  width: 500,
  height: 500,
  centerX: 250,
  centerY: 250,
  bools: [true, true],
}
/**
 * R1 种群：基础个体
 * 行为：趋向群体中心，受到 R2 修正力场的引导
 */
export const r1Race: Race<Scope> = new Race(
  'r1',
  scope,
  {
    default_state(entity: Entity<Scope>): VectorN {
      // 初始位置随机分布在画布内
      return VectorN.random(2, 0, 500)
    },

    update(entity: Entity<Scope>, state: VectorN, action: VectorN): VectorN {
      // 限制最大步长，防止“瞬移”
      const speedLimit = 5
      const vx = Math.max(-speedLimit, Math.min(speedLimit, action.values[0]!))
      const vy = Math.max(-speedLimit, Math.min(speedLimit, action.values[1]!))
      return new VectorN([state.values[0]! + vx, state.values[1]! + vy])
    },

    reward(entity: Entity<Scope>, state: VectorN, world: World): number {
      const group = world.get_entitys('r1')
      const scope = world.get_scope<Scope>('r1')
      // return Math.random() * 5
      if (group.length < 2) return 0

      const x = state.values[0]!
      const y = state.values[1]!

      let sum = 0

      for (const other of group) {
        if (other === entity) continue

        const ox = other.cur_exp.state.values[0]!
        const oy = other.cur_exp.state.values[1]!

        const d = Math.sqrt((ox - x) ** 2 + (oy - y) ** 2)

        // ✔ 越近越好（纯吸引）
        sum += 1 / (d + 1e-3)
      }

      // 3. 边缘约束（防止跑出画布找不到家）
      let boundaryPenalty = 0
      const margin = 50
      if (x < margin || x > scope.width - margin || y < margin || y > scope.height - margin) {
        boundaryPenalty = -200
      }

      return sum + boundaryPenalty
    },
    rectify_underlying_entity_reward(
      this_entity: Entity<Scope>,
      under_entity: Entity<any>,
      world: World,
    ): number {
      return 0 // R1 不修饰下层
    },
  },
  // () => new HistoryBasedStrategy(10),
  // () => new VelocityStrategyV2(0.2, 0.05),
  () => new VelocityStrategy(0.3, 0.3, 0.05),
)

/**
 * r2 种群：高级组织者/力场
 * 行为：控制半径和强度，诱导 R1 形成完美的圆环
 */
// export const r2Race: Race<Scope> = new Race(
//   'r2',
//   scope,
//   {
//     default_state(entity: Entity<Scope>): VectorN {
//       // 随机初始化 6 个参数：w1, w2, w3, w4, w5, w6
//       return VectorN.random(7, -10, 10)
//     },
//
//     update(entity: Entity<Scope>, state: VectorN, action: VectorN): VectorN {
//       const next = state.add(action)
//
//       // 【可选】权重正则化：防止 R2 走火入魔把权重加到无限大
//       // 把每个权重限制在 [-10, 10] 之间
//       for (let i = 0; i < next.values.length; i++) {
//         next.values[i] = Math.max(-10, Math.min(10, next.values[i]!))
//       }
//       return next
//     },
//
//     reward(entity: Entity<Scope>, state: VectorN, world: World): number {
//       const cells = entity.cells
//       if (cells.length < 3) return -1000
//
//       // 1. 计算质心
//       const cx = cells.reduce((s, e) => s + e.cur_exp.state.values[0]!, 0) / cells.length
//       const cy = cells.reduce((s, e) => s + e.cur_exp.state.values[1]!, 0) / cells.length
//
//       let radiusVariance = 0
//       let min_dist_sum = 0
//       const radii: number[] = []
//
//       // 2. 计算半径和最短邻居距离
//       for (let i = 0; i < cells.length; i++) {
//         const e1 = cells[i]!
//         const x1 = e1.cur_exp.state.values[0]!
//         const y1 = e1.cur_exp.state.values[1]!
//
//         // 记录到中心的距离
//         const r = Math.sqrt((x1 - cx) ** 2 + (y1 - cy) ** 2)
//         radii.push(r)
//
//         // 计算当前细胞离它最近的兄弟有多远
//         let min_d = Infinity
//         for (let j = 0; j < cells.length; j++) {
//           if (i === j) continue
//           const e2 = cells[j]!
//           const d = Math.sqrt((x1 - e2.cur_exp.state.values[0]!) ** 2 + (y1 - e2.cur_exp.state.values[1]!) ** 2)
//           if (d < min_d) min_d = d
//         }
//         min_dist_sum += min_d
//       }
//
//       // 圆形度惩罚：半径的方差
//       const avgRadius = radii.reduce((a, b) => a + b, 0) / radii.length
//       radiusVariance = radii.reduce((s, r) => s + (r - avgRadius) ** 2, 0) / radii.length
//
//       // 拥挤度惩罚：理想状态下，N个细胞均匀分布在周长上，间距应为 (2*PI*R)/N
//       const avg_min_dist = min_dist_sum / cells.length
//       const ideal_dist = (2 * Math.PI * avgRadius) / cells.length
//       // 如果实际平均最小间距远小于理想间距，说明发生了抱团
//       const dist_penalty = Math.abs(ideal_dist - avg_min_dist)
//
//       // 综合评分：底分 10000 - 形状不圆的惩罚 - 抱团的惩罚
//       return 10000 - Math.sqrt(radiusVariance) * 50 - dist_penalty * 200
//     },
//
//     /**
//      * 修正 R1 的奖励：
//      * 这里的 state 是 R1 的位置，我们需要让 R1 意识到“只有在半径边缘才是安全的”
//      */
//     rectify_underlying_entity_reward(
//       this_entity: Entity<Scope>,
//       under_entity: Entity<any>,
//       world: World,
//     ): number {
//       if (under_entity.race_name !== 'r1') return 0
//       const cell_state = under_entity.cur_exp.state
//       const weights = this_entity.cur_exp.state.values
//       const x = cell_state.values[0]!
//       const y = cell_state.values[1]!
//       // 获取当前作用域中心和宽高
//       const scope = world.get_scope<Scope>('r2')
//
//       // 1. 获取基础物理坐标（归一化）
//       const nx = (cell_state.values[0]! - scope.centerX) / (scope.width / 2)
//       const ny = (cell_state.values[1]! - scope.centerY) / (scope.height / 2)
//
//       // 2. 环境固有的物理场（该坐标的实体密度）。这是宇宙法则，不是R2刻意为之
//       let density = 0
//       for (const other of this_entity.cells) {
//         const ox = (other.cur_exp.state.values[0]! - scope.centerX) / (scope.width / 2)
//         const oy = (other.cur_exp.state.values[1]! - scope.centerY) / (scope.height / 2)
//         const distSq = (nx - ox) ** 2 + (ny - oy) ** 2
//         density += 1 / (distSq + 0.01) // 纯粹的平方反比物理场
//       }
//       density = density / this_entity.cells.length // 归一化场强
//
//       // 3. 提取空间特征向量 (Sensors)
//       const features = [
//         nx,             // [0] 一次项 X
//         ny,             // [1] 一次项 Y
//         nx * nx,        // [2] 二次项 X^2 (涌现环形的必要条件)
//         ny * ny,        // [3] 二次项 Y^2 (涌现环形的必要条件)
//         nx * ny,        // [4] 交叉项 XY
//         density         // [5] 空间场密度
//       ]
//
//       // 4. 纯粹的代数映射： Reward = W · Features
//       let out = 0
//       for (let i = 0; i < 6; i++) {
//         out += weights[i]! * features[i]!
//       }
//
//       // 用 tanh 保证能量守恒，不超过环境容量上限
//       return 50 * Math.tanh(out)
//     },
//   },
//   // () => new HistoryBasedStrategy(1, 10),
//   () => new VelocityStrategy(1, 0.3),
// )

export const r2Race: Race<Scope> = new Race(
  'r2',
  scope,
  {
    default_state(entity: Entity<Scope>): VectorN {
      return VectorN.random(16, -5, 5)
    },

    update(entity: Entity<Scope>, state: VectorN, action: VectorN): VectorN {
      const next = state.add(action)
      return next
    },

    reward(entity: Entity<Scope>, state: VectorN, world: World): number {
      const cells = entity.cells
      if (cells.length < 3) return -1000

      const targetRadius = 50

      // 1. 获取质心
      const cx = cells.reduce((s, e) => s + e.cur_exp.state.values[0]!, 0) / cells.length
      const cy = cells.reduce((s, e) => s + e.cur_exp.state.values[1]!, 0) / cells.length

      let radiusErrorSum = 0
      let minDistSum = 0

      // 2. 遍历计算物理表现
      for (let i = 0; i < cells.length; i++) {
        const e1 = cells[i]!
        const x1 = e1.cur_exp.state.values[0]!
        const y1 = e1.cur_exp.state.values[1]!

        // 计算半径误差
        const r = Math.sqrt((x1 - cx) ** 2 + (y1 - cy) ** 2)
        radiusErrorSum += Math.abs(r - targetRadius)

        // 计算最小间距
        let min_d = Infinity
        for (let j = 0; j < cells.length; j++) {
          if (i === j) continue
          const e2 = cells[j]!
          const d = Math.sqrt(
            (x1 - e2.cur_exp.state.values[0]!) ** 2 + (y1 - e2.cur_exp.state.values[1]!) ** 2,
          )
          if (d < min_d) min_d = d
        }
        minDistSum += min_d
      }

      // 半径误差惩罚
      const avgRadiusError = radiusErrorSum / cells.length
      // 拥挤惩罚（对比理想均匀间距）
      const ideal_dist = (2 * Math.PI * targetRadius) / cells.length
      const avg_min_dist = minDistSum / cells.length
      const dist_penalty = Math.abs(ideal_dist - avg_min_dist)

      // R2 拿高分的条件：绝对贴合目标半径，且绝对均匀
      return 5000 - avgRadiusError * 50 - dist_penalty * 200
    },

    /**
     * 【纯物理场映射：不再有任何业务逻辑的 IF/ELSE】
     */
    rectify_underlying_entity_reward(
      this_entity: Entity<Scope>,
      under_entity: Entity<any>,
      world: World,
    ): number {
      if (under_entity.race_name !== 'r01') return 0
      const cell_state = this_entity.cur_exp.state
      const scope = world.get_scope<Scope>('r2')
      let features = [...under_entity.cur_exp.state.values]
      const neighbors = get_nearest_neighbors(under_entity, world, 3, 'r1')
      neighbors.forEach(n => {
        features.push(...n.cur_exp.state.values)
      })
      features = features.map(f => f / 500)
      const weights = cell_state.values
      // 4. 纯粹的代数映射： Reward = W · Features
      const outs = []
      for (let i = 0; i < features.length; i++) {
        outs.push(weights[i]! * features[i]!)
      }
      let out = 0
      for (let i = 0; i < outs.length; i++) {
        out += weights[i + outs.length-1]! * outs[i]!
      }

      // 用 tanh 保证能量守恒，不超过环境容量上限
      return 50 * Math.tanh(out)
    },
  },
  // () => new VelocityStrategy(1, 0.3),
  () => new HistoryBasedStrategy(1, 50),
)

function get_nearest_neighbors(
  this_entity: Entity<any>,
  world: World,
  n: number,
  race_name: string = 'r1',
): Entity<any>[] {
  const others = world.get_entitys(race_name)
  const my_pos = this_entity.cur_exp.state

  return others
    .filter(other => other !== this_entity) // 排除自己
    .map(other => ({
      entity: other,
      dist: my_pos.distance(other.cur_exp.state), // 假设 VectorN 有静态 distance 方法
    }))
    .sort((a, b) => a.dist - b.dist) // 按距离升序排列
    .slice(0, n) // 取前 N 个
    .map(item => item.entity)
}

export const xorRace: Race<Scope> = new Race(
  'xor',
  scope,
  {
    default_state(): VectorN {
      // pressure
      return VectorN.random(1, 0, 100)
    },

    update(entity: Entity<Scope>, state: VectorN, action: VectorN): VectorN {
      return state.add(action)
    },

    reward(entity: Entity<Scope>, state: VectorN, world: World): number {
      const scope = world.get_scope<Scope>('xor')

      if (entity.cells.length === 0) return -100
      //----------------------------------------
      // xor 真值
      //----------------------------------------

      const target = scope.bools[0] !== scope.bools[1]

      const r2 = entity.cells[0]!

      if (r2.cells.length === 0) return -100
      const radius = r2.cur_exp.state.values[0]!
      if (target) {
        return (radius - 80) / 100
      }
      return (80 - radius) / 100
    },

    rectify_underlying_entity_reward(
      this_entity: Entity<Scope>,
      under_entity: Entity<any>,
      world: World,
    ): number {
      if (under_entity.race_name !== 'r2') return 0
      const cell_state = under_entity.cur_exp.state
      //----------------------------------------
      // pressure
      //----------------------------------------

      const pressure = this_entity.cur_exp.state.values[0] ?? 0

      const radius = cell_state.values[0] ?? 0
      const x = pressure - radius

      return (100 - x) / 100
    },
  },
  // () => new VelocityStrategy(0.5, 0.3),
  () => new HistoryBasedStrategy(0.5, 5),
)
//不干活的
export const topRace: Race<any> = new Race(
  'top',
  {} as any,
  {
    default_state(entity: Entity<any>): VectorN {
      return new VectorN([])
    },
    update(entity: Entity<any>, state: VectorN, action: VectorN): VectorN {
      return state
    },

    reward(entity: Entity<any>, state: VectorN, world: World): number {
      return 0
    },

    rectify_underlying_entity_reward(
      this_entity: Entity<any>,
      under_entity: Entity<any>,
      world: World,
    ): number {
      return 0
    },
  },
  () => new BestExperienceStrategy(),
)
