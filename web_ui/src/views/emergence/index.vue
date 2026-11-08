<script setup lang="ts">
import CanvasView from './CanvasView.vue'

import ScorePanel from './ScorePanel.vue'

import { Layer, Simulation } from './simulation.ts'
import { r1Race, r2Race, type Scope } from '@/views/emergence/core/entity.ts'

const layers = [new Layer(1, r2Race, 10), new Layer(30, r1Race, 1)]
const sim = new Simulation(layers)
sim.run(0.16)
const xor_datas = [
  [[0, 0], 0],
  [[0, 1], 1],
  [[1, 0], 1],
  [[1, 1], 0],
]

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

let i = 0
// const xor_entity = sim.world.get_entitys('xor')[0]!
// const xor_scope = sim.world.get_scope<Scope>('xor')
// const interval = 1000
// setInterval(async () => {
//   const [inputs, out] = xor_datas[i % 4]!
//   xor_scope.bools = [inputs[0] > 0.5, inputs[1] > 0.5]
//   await sleep(interval / 2)
//   const o = xor_entity.cur_exp.reward
//   console.log(inputs, out, o)
//   i++
// }, interval)




const entitys = sim.get_entitys()
</script>

<template>
  <div class="h-screen flex">
    <CanvasView :simulation="sim" />
    <ScorePanel :entities="entitys" />
  </div>
</template>
