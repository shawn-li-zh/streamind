<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import type { Simulation } from '@/views/emergence/simulation'
import type { Scope } from '@/views/emergence/core/entity.ts'

const props = defineProps<{
  simulation: Simulation
}>()
const sim = props.simulation
const scope: Scope = sim.world.get_scope('r1')
const canvas = ref<HTMLCanvasElement>()

let frameId = 0

function draw() {
  const ctx = canvas.value?.getContext('2d')
  if (!ctx) {
    return
  }

  ctx.clearRect(0, 0, scope.width, scope.height)

  drawWorld(ctx)

  drawE1(ctx)

  drawE2(ctx)
}

function drawWorld(ctx: CanvasRenderingContext2D) {
  ctx.strokeStyle = '#94a3b8'

  ctx.strokeRect(0, 0, scope.width, scope.height)
}

function drawE1(ctx: CanvasRenderingContext2D) {
  for (const e of sim.world.get_entitys('r1')) {
    ctx.beginPath()
    const [x, y] = e.cur_exp.state.values
    ctx.arc(x!, y!, 5, 0, Math.PI * 2)

    ctx.fillStyle = '#3b82f6'

    ctx.fill()
  }
}

function drawE2(ctx: CanvasRenderingContext2D) {
  for (const e of sim.world.get_entitys('r2')) {
    if (e.cells.length == 0) continue

    const cx = e.cells.reduce((s, e) => s + e.cur_exp.state.values[0]!, 0) / e.cells.length

    const cy = e.cells.reduce((s, e) => s + e.cur_exp.state.values[1]!, 0) / e.cells.length

    ctx.beginPath()

    ctx.arc(cx, cy, 8, 0, Math.PI * 2)

    ctx.fillStyle = '#ef4444'

    ctx.fill()
  }
}

function loop(){
  draw()
  frameId = requestAnimationFrame(loop)
}
onMounted(() => {
  loop()
})
onUnmounted(() => {
  cancelAnimationFrame(frameId)
})
</script>

<template>
  <canvas
    ref="canvas"
    :width="scope.width"
    :height="scope.height"
    class="border rounded bg-white"
  />
</template>
