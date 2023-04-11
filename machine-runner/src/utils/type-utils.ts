export type DeepReadonly<T> = {
  readonly [P in keyof T]: T[P] extends Record<string, unknown> ? DeepReadonly<T[P]> : T[P]
}

export type NonZeroTuple<T> = [T, ...T[]]

export type ExtendsThenTransform<A, B, T = true, F = false> = [A] extends [B] ? T : F

// utilities from https://github.com/type-challenges/type-challenges
export type Equal<X, Y> = (<T>() => T extends X ? 1 : 2) extends <T>() => T extends Y ? 1 : 2
  ? true
  : false
export type NotEqual<X, Y> = true extends Equal<X, Y> ? false : true

export type Expect<T extends true> = T

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type NotAnyOrUnknown<T> = any extends T ? never : T

type SerializablePrimitive = number | string | boolean | null | undefined

export type SerializableObject = {
  [_: string]:
    | SerializablePrimitive
    | SerializablePrimitive[]
    | SerializableObject
    | SerializableObject[]
  [_: number]: never
  [_: symbol]: never
}
