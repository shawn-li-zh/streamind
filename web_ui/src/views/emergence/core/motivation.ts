// motivation/Motivation.ts

import type { Experience } from './domain.ts'
import type { LearningStrategy } from '@/views/emergence/core/learning.ts'

export class Motivation {
  history: Experience[] = []

  constructor(private learner: LearningStrategy) {}

  record(experience: Experience) {
    this.history.push(experience)

    if (this.history.length > 50) {
      this.history.shift()
    }
  }

  nextAction() {
    return this.learner.nextControl(this.history)
  }
}
