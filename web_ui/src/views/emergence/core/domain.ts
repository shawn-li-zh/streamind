// domain/Entity.ts

import { VectorN } from '@/views/emergence/core/VectorN.ts'
import type { LearningStrategy } from '@/views/emergence/core/learning.ts'

export interface RaceTrait<S> {
  //默认初始状态
  default_state(entity: Entity<S>): VectorN

  //状态更新 返回新状态
  update(entity: Entity<S>, state: VectorN, action: VectorN): VectorN

  //获取当前状态的奖励
  reward(entity: Entity<S>, state: VectorN, world: World): number

  //修正下层实体的状态奖励
  rectify_underlying_entity_reward(
    this_entity: Entity<S>,
    under_entity: Entity<any>,
    world: World,
  ): number
}

export class Race<S> {
  name: string
  entity_id: number

  constructor(
    name: string,
    public scope: S,
    public inner: RaceTrait<S>,
    private learn_builder: () => LearningStrategy,
  ) {
    this.name = name
    this.entity_id = 0
    this.inner = inner
  }

  new_entity(upper_entity: Entity<S>): Entity<S> {
    const learn = this.learn_builder()
    const entity = new Entity(this.entity_id++, this, upper_entity, learn)
    upper_entity.cells.push(entity)
    return entity
  }
}

export class Entity<S> {
  public cur_exp: Experience
  public score: number = 0
  public cells: Entity<S>[] = []
  public race_name: string

  constructor(
    public id: number,
    private race: Race<S>,
    public upper_entity: Entity<S>,
    private learn: LearningStrategy,
  ) {
    this.race_name = race.name
    const state = race.inner.default_state(this)
    this.cur_exp = {
      state: state,
      action: VectorN.zero(state.dim()),
      reward: 0,
    }
  }

  next_action() {
    return this.learn.next_action(this.cur_exp)
  }

  step(action: VectorN, world: World): number {
    const lr = this.learn.lr
    const old_exp = this.cur_exp
    const old_state = old_exp.state
    const new_state = this.race.inner.update(this, old_state, action.scale(lr))
    let reward = this.race.inner.reward(this, this.cur_exp.state, world)
    // 让上层实体修正下层实体的状态奖励
    const reward_fix = this.upper_entity.race.inner.rectify_underlying_entity_reward(
      this.upper_entity,
      this,
      world,
    )
    reward += reward_fix
    this.cur_exp = {
      state: new_state,
      action,
      reward,
    }
    this.score += reward
    return reward
  }
}

export interface Experience {
  state: VectorN
  action: VectorN
  reward: number
}

export class World {
  public entitys: Map<string, Entity<any>[]> = new Map()
  public scopes: Map<string, any> = new Map()

  get_entitys(race_name: string): Entity<any>[] {
    const entitys = this.entitys.get(race_name) || []
    this.entitys.set(race_name, entitys)
    return entitys
  }

  get_scope<T>(race_name: string): T {
    const scope = this.scopes.get(race_name)
    if (!scope) {
      throw new Error(`scope ${race_name} not found`)
    }
    return scope
  }
}
