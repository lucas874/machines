import { check_swarm, check_projection, check_wwf_swarm, get_wwf_sub, compose_subs, revised_projection } from '../pkg/machine_check.js'

export type Protocol<Label> = {
  initial: string
  transitions: { source: string; target: string; label: Label }[]
}
export type SwarmLabel = {
  cmd: string
  logType: string[]
  role: string
}

export type MachineLabel =
  | { tag: 'Execute'; cmd: string; logType: string[] }
  | { tag: 'Input'; eventType: string }

export type SwarmProtocolType = Protocol<SwarmLabel>
export type MachineType = Protocol<MachineLabel>

export type Subscriptions = Record<string, string[]>

export type Result = { type: 'OK' } | { type: 'ERROR'; errors: string[] }

export type ResultData = { type: 'OK'; data: string } | { type: 'ERROR'; errors: string[] }

export type CompositionInput = { protocol: SwarmProtocolType, subscription: Subscriptions, interface: string | null }

export type CompositionInputVec = CompositionInput[]


export function checkSwarmProtocol(proto: SwarmProtocolType, subscriptions: Subscriptions): Result {
  const p = JSON.stringify(proto)
  const s = JSON.stringify(subscriptions)
  const result = check_swarm(p, s)
  return JSON.parse(result)
}

export function checkProjection(
  swarm: SwarmProtocolType,
  subscriptions: Subscriptions,
  role: string,
  machine: MachineType,
): Result {
  const sw = JSON.stringify(swarm)
  const sub = JSON.stringify(subscriptions)
  const m = JSON.stringify(machine)
  const result = check_projection(sw, sub, role, m)
  return JSON.parse(result)
}

export function checkWWFSwarmProtocol(proto: SwarmProtocolType, subscriptions: Subscriptions): Result {
  const p = JSON.stringify(proto)
  const s = JSON.stringify(subscriptions)
  const result = check_wwf_swarm(p, s)
  return JSON.parse(result)
}

export function getWWFSub(proto: SwarmProtocolType): ResultData {
  const p = JSON.stringify(proto)
  const result = get_wwf_sub(p)
  return JSON.parse(result)
}

export function composeTwoSubs(proto1: SwarmProtocolType, subscriptions1: Subscriptions, proto2: SwarmProtocolType, subscriptions2: Subscriptions, swarm_interface: string): ResultData {
  const ps: CompositionInputVec = [{protocol: proto1, subscription: subscriptions1, interface: null}, {protocol: proto2, subscription: subscriptions2, interface: swarm_interface}]
  const ps_str = JSON.stringify(ps)
  const result = compose_subs(ps_str)
  return JSON.parse(result)
}

export function composeSubs(protos: CompositionInputVec): ResultData {
  const ps = JSON.stringify(protos)
  const result = compose_subs(ps);
  return JSON.parse(result)
}

export function revisedProjection(proto: SwarmProtocolType, subscriptions: Subscriptions, role: string): ResultData {
  const p = JSON.stringify(proto)
  const s = JSON.stringify(subscriptions)
  const result = revised_projection(p, s, role)
  return JSON.parse(result)
}