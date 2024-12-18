import { check_swarm, check_projection, check_wwf_swarm, exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub, check_composed_projection, revised_projection, project_combine, compose_protocols, project_combine_all } from '../pkg/machine_check.js'

//, check_composed_projection, get_wwf_sub, compose_subs, revised_projection, project_combine, project_combine_all, compose_protocols } from '../pkg/machine_check.js'
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

//export type ResultData = { type: 'OK'; data: string } | { type: 'ERROR'; errors: string[] }
//export type CompositionInput = { protocol: SwarmProtocolType, subscription: Subscriptions, interface: string | null }
//export type CompositionInputVec = CompositionInput[]

export type ResultData<Data> = { type: 'OK'; data: Data } | { type: 'ERROR'; errors: string[] }
export type CompositionComponent = {protocol: SwarmProtocolType, subscriptions: Subscriptions, interface: string | null }
export type InterfacingSwarms = CompositionComponent[]
export type Granularity =
  | "Fine"
  | "Medium"
  | "Coarse"

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

export function checkWWFSwarmProtocol(protos: InterfacingSwarms, subscriptions: Subscriptions): Result {
  const p = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const result = check_wwf_swarm(p, s)
  return JSON.parse(result)
}

export function exactWWFSubscriptions(protos: InterfacingSwarms): ResultData<Subscriptions> {
  const p = JSON.stringify(protos)
  const result = exact_weak_well_formed_sub(p);
  return JSON.parse(result)
}

export function overapproxWWFSubscriptions(protos: InterfacingSwarms): ResultData<Subscriptions> {
  const p = JSON.stringify(protos)
  const result = overapproximated_weak_well_formed_sub(p);
  return JSON.parse(result)
}

export function checkComposedProjection(
  protos: InterfacingSwarms,
  subscriptions: Subscriptions,
  role: string,
  machine: MachineType,
): Result {
  const ps = JSON.stringify(protos)
  const sub = JSON.stringify(subscriptions)
  const m = JSON.stringify(machine)
  const result = check_composed_projection(ps, sub, role, m)
  return JSON.parse(result)
}

export function revisedProjection(
  proto: SwarmProtocolType,
  subscriptions: Subscriptions,
  role: string
): ResultData<MachineType> {
  const p = JSON.stringify(proto)
  const s = JSON.stringify(subscriptions)
  const result = revised_projection(p, s, role)
  return JSON.parse(result)
}

export function projectCombineMachines(protos: InterfacingSwarms, subscriptions: Subscriptions, role: string): ResultData<MachineType> {
  const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const result = project_combine(ps, s, role);
  return JSON.parse(result)
}

export function composeProtocols(protos: InterfacingSwarms): ResultData<SwarmProtocolType> {
  const ps = JSON.stringify(protos)
  const result = compose_protocols(ps)
  return JSON.parse(result)
}

export function projectAll(protos: InterfacingSwarms, subscriptions: Subscriptions): ResultData<MachineType[]> {
  const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const result = project_combine_all(ps, s)
  return JSON.parse(result)
}