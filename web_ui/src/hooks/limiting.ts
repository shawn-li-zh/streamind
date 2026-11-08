export function useDebounce<T, F extends (...args: any[]) => T>(func: F, delay: number): (...args: Parameters<F>) => Promise<T> {
  let timer: number | null = null;
  return function (...args: any[]) {
    if (timer) {
      clearTimeout(timer);
    }
    return new Promise(resolve => {
      const t = setTimeout(() => {
        const res = func(...args);
        resolve(res);
      }, delay);
      timer = t as unknown as number;
    });
  };
}
