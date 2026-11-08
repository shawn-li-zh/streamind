// simulation/Simulation.ts

import { Entity, Race, World } from '@/views/emergence/core/domain.ts'
import { topRace } from '@/views/emergence/core/entity.ts'
import { VectorN } from '@/views/emergence/core/VectorN.ts'

export class Simulation {
  world = new World()
  private layers: Layer<any>[]

  constructor(layers: Layer<any>[]) {
    this.layers = layers
    this.init(layers)
  }

  get_entitys() {
    return [...this.world.entitys.values()].flatMap(entitys => entitys)
  }

  private init(layers: Layer<any>[]) {
    console.log('init')
    const top = topRace.new_entity(new Entity(0, topRace, {} as any, {} as any))

    let upper_entitys = [top]
    for (const layer of layers) {
      this.world.scopes.set(layer.race.name, layer.race.scope)
      const entitys = this.world.get_entitys(layer.race.name)
      const sub_entitys = []
      for (const upperEntity of upper_entitys) {
        sub_entitys.push(...layer.init(upperEntity))
      }
      upper_entitys = sub_entitys
      entitys.push(...sub_entitys)
    }
    //
    // 先随机扰动一次
    for (const entitys of this.world.entitys.values()) {
      entitys.forEach(e => {
        const dim = e.cur_exp.state.dim()
        const action = VectorN.random(dim, -1, 1).normalize()
        e.step(action, this.world)
      })
    }
  }

  async run() {
    const counts: number[] = this.layers.map(() => 0)
    const layers = this.layers.reverse()
    while (true) {
      for (let i = 0; i < layers.length; i++) {
        if (i == 0) {
          if (counts[0]! % 1000 == 0) {
            await sleep(10)
          }
        }
        const layer = layers[i]!
        const time_interval = i == 0 ? 1 : layer.time_interval
        if ((counts[i - 1] || 0) % time_interval == 0) {
          layer.step(this.world)
          counts[i]!++
          continue
        }
        break
      }
    }
  }
}

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

export class Layer<S> {
  private entitys: Entity<S>[] = []

  constructor(
    private num: number,
    public race: Race<S>,
    public time_interval = 50,
  ) {}

  init(upper_entity: Entity<any>) {
    for (let i = 0; i < this.num; i++) {
      const entity = this.race.new_entity(upper_entity)
      this.entitys.push(entity)
    }
    return this.entitys
  }

  step(world: World) {
    const actions = []
    // 第一阶段
    // 所有人观察同一个世界
    for (const e of this.entitys) {
      actions.push(e.next_action())
    }
    // 第二阶段
    // 统一更新
    for (const e of this.entitys) {
      const action = actions.shift()!
      e.step(action, world)
    }
  }
}
