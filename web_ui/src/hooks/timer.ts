// useTimer.ts
import { computed, onUnmounted, ref } from 'vue'

export function useTimer(isStop: () => boolean, formatter: (ms: number) => string) {
  const start = Date.now()
  const now = ref(Date.now())
  const timer = ref<number>()

  const duration = computed(() => {
    const diff = Math.max(0, now.value - start)
    return formatter(diff)
  })
  const pause = () => clearInterval(timer.value)
  const resume = () => {
    timer.value = setInterval(() => {
      if (isStop()) pause()
      now.value = Date.now()
    }, 200)
  }
  resume()
  onUnmounted(pause)
  return { duration, pause, resume }
}
