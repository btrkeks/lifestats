/**
 * The plan workout done least recently, as an index into `names`: never finished before finished, then the
 * one finished longest ago, the earlier in the plan on a tie. The workout in progress counts as done now.
 * Undefined for a plan with fewer than two workouts, where there is nothing to choose between.
 */
export function upNextIndex(
  names: readonly string[],
  lastFinishedMs: ReadonlyMap<string, number>,
  inProgress?: string,
): number | undefined {
  if (names.length < 2) {
    return undefined;
  }
  const doneAt = (name: string) => (name === inProgress ? Infinity : (lastFinishedMs.get(name) ?? -Infinity));
  let next = 0;
  names.forEach((name, index) => {
    if (doneAt(name) < doneAt(names[next]!)) {
      next = index;
    }
  });
  return next;
}
