import { check_swarm, check_projection, check_wwf_swarm, exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub, check_composed_projection, revised_projection, project_combine, compose_protocols, project_combine_all, projection_information, projection_information_new } from '../pkg/machine_check.js'

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

export type ResultData<Data> = { type: 'OK'; data: Data } | { type: 'ERROR'; errors: string[] }
export type CompositionComponent = {protocol: SwarmProtocolType, interface: string | null }
export type InterfacingSwarms = CompositionComponent[]
export type Granularity =
  | 'Fine'
  | 'Medium'
  | 'Coarse'
  | 'TwoStep'
export type SucceedingNonBranchingJoining = Record<string, Set<string>>;
export type ProjectionAndSucceedingMap = {
    projection: MachineType,
    branches: SucceedingNonBranchingJoining,
    specialEventTypes: Set<string>,
}
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

export function exactWWFSubscriptions(protos: InterfacingSwarms, subscriptions: Subscriptions): ResultData<Subscriptions> {
  const p = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const result = exact_weak_well_formed_sub(p, s);
  return JSON.parse(result)
}

export function overapproxWWFSubscriptions(protos: InterfacingSwarms, subscriptions: Subscriptions, granularity: Granularity): ResultData<Subscriptions> {
  const p = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const g = JSON.stringify(granularity)
  const result = overapproximated_weak_well_formed_sub(p, s, g);
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

export function projectionAndInformation(protos: InterfacingSwarms, subscriptions: Subscriptions, role: string): ResultData<ProjectionAndSucceedingMap> {
  const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const result = JSON.parse(projection_information(ps, s, role));
  if (result.type === "ERROR") {
    return result
  } else {
    const data = {...result.data, specialEventTypes: new Set<string>(result.data.specialEventTypes) }
    return {type: "OK", data: data}
  }

}

export function projectionAndInformationNew(protos: InterfacingSwarms, subscriptions: Subscriptions, role: string, machine: MachineType, k: number): ResultData<ProjectionAndSucceedingMap> {
  const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const m = JSON.stringify(machine)
  const result = JSON.parse(projection_information_new(ps, s, role, m, k.toString()));
  if (result.type === "ERROR") {
    return result
  } else {
    const data = {...result.data, specialEventTypes: new Set<string>(result.data.specialEventTypes) }
    return {type: "OK", data: data}
  }

}