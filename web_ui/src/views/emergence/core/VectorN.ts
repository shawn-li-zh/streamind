// core/VectorN.ts

export class VectorN {
  constructor(public values: number[]) {}

  dim() {
    return this.values.length
  }

  clone() {
    return new VectorN([...this.values])
  }

  add(v: VectorN): VectorN {
    const values = this.values.map((x, i) => x + v.values[i]!)
    return new VectorN(values)
  }

  sub(v: VectorN): VectorN {
    const values = this.values.map((x, i) => x - v.values[i]!)
    return new VectorN(values)
  }

  scale(n: number) {
    const values = this.values.map(x => x * n)
    return new VectorN(values)
  }

  map(f: (x: number) => number) {
    const values = this.values.map(f)
    return new VectorN(values)
  }

  norm() {
    return Math.sqrt(this.values.reduce((s, v) => s + v * v, 0))
  }

  normSquared() {
    return this.values.reduce((s, v) => s + v * v, 0)
  }

  normalize(): VectorN {
    const len = this.norm()

    if (len < 1e-6) {
      return this.clone()
    }

    return new VectorN(this.values.map(v => v / len))
  }

  dot(v: VectorN) {
    return this.values.reduce((s, x, i) => s + x * v.values[i]!, 0)
  }

  // 裁剪
  clip(min: number, max: number) {
    const clippedValues = this.values.map(v => Math.min(Math.max(v, min), max))
    return new VectorN(clippedValues)
  }

  /**
   * 计算与另一个向量的距离平方（性能更好）
   */
  distanceSquared(v: VectorN): number {
    let sum = 0
    for (let i = 0; i < this.values.length; i++) {
      const diff = this.values[i]! - v.values[i]!
      sum += diff * diff
    }
    return sum
  }

  /**
   * 计算与另一个向量的欧氏距离
   */
  distance(v: VectorN): number {
    return Math.sqrt(this.distanceSquared(v))
  }

  static random(dim: number, min: number = 0, max: number = 1) {
    return new VectorN(Array.from({ length: dim }, () => Math.random() * (max - min) + min))
  }

  static zero(dim: number) {
    return new VectorN(Array.from({ length: dim }, () => 0))
  }
}
