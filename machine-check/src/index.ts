import { check_swarm, check_projection, check_wwf_swarm, exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub, check_composed_projection,
  revised_projection, project_combine, compose_protocols, projection_information, projection_information_new,
  CheckResult, Machine, SwarmProtocol, Subscriptions, InterfacingSwarms as InterfacingSwarmsInner, Role, DataResult, Granularity,
  ProjectionInfo } from '../pkg/machine_check.js'
export { Machine as MachineType, SwarmProtocol as SwarmProtocolType, Subscriptions, Role, CheckResult as Result, DataResult, Granularity, ProjectionInfo }
export type InterfacingSwarms = InterfacingSwarmsInner<Role>;
/* export type Protocol<Label> = {
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
  | 'TwoStep' */
/* export type SucceedingNonBranchingJoining = Record<string, Set<string>>;
export type ProjectionAndSucceedingMap = {
  projection: Machine,
  branches: SucceedingNonBranchingJoining,
  specialEventTypes: Set<string>,
} */
export function checkSwarmProtocol(proto: SwarmProtocol, subscriptions: Subscriptions): CheckResult {
  const p = JSON.stringify(proto)
  const s = JSON.stringify(subscriptions)
  const result = check_swarm(p, s)
  return JSON.parse(result)
}

export function checkProjection(
  swarm: SwarmProtocol,
  subscriptions: Subscriptions,
  role: string,
  machine: Machine,
): CheckResult {
  const sw = JSON.stringify(swarm)
  const sub = JSON.stringify(subscriptions)
  const m = JSON.stringify(machine)
  const result = check_projection(sw, sub, role, m)
  return JSON.parse(result)
}

export function checkWWFSwarmProtocol(protos: InterfacingSwarms, subscriptions: Subscriptions): CheckResult {
  return check_wwf_swarm(protos, JSON.stringify(subscriptions))
}

export function exactWWFSubscriptions(protos: InterfacingSwarms, subscriptions: Subscriptions): DataResult<Subscriptions> {
  return exact_weak_well_formed_sub(protos, JSON.stringify(subscriptions));
}

export function overapproxWWFSubscriptions(protos: InterfacingSwarms, subscriptions: Subscriptions, granularity: Granularity): DataResult<Subscriptions> {
  //const p = JSON.stringify(protos)
  //const s = JSON.stringify(subscriptions)
  //const g = JSON.stringify(granularity)
  //const result = overapproximated_weak_well_formed_sub(protos, JSON.stringify(subscriptions), granularity);
  //return JSON.parse(result)
  //const result: any = 1
  //return result
  return overapproximated_weak_well_formed_sub(protos, JSON.stringify(subscriptions), granularity);
}
/*
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
*/
export function revisedProjection(
  proto: SwarmProtocol,
  subscriptions: Subscriptions,
  role: Role,
  minimize: boolean
): DataResult<Machine> {
/*   const p = JSON.stringify(proto)
  const s = JSON.stringify(subscriptions)
  const m = minimize.toString()
  const result = revised_projection(p, s, role, m)
  return JSON.parse(result) */
  return revised_projection(proto, JSON.stringify(subscriptions), role, minimize)
}

export function projectCombineMachines(protos: InterfacingSwarms, subscriptions: Subscriptions, role: string, minimize: boolean): DataResult<Machine> {
/*   const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const m = minimize.toString()
  const result = project_combine(ps, s, role, m);
  return JSON.parse(result) */
  return project_combine(protos, JSON.stringify(subscriptions), role, minimize)
}

/*
export function composeProtocols(protos: InterfacingSwarms): ResultData<SwarmProtocolType> {
  const ps = JSON.stringify(protos)
  const result = compose_protocols(ps)
  return JSON.parse(result)
}

export function projectAll(protos: InterfacingSwarms, subscriptions: Subscriptions, minimize: boolean): ResultData<MachineType[]> {
  const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const m = minimize.toString()
  const result = project_combine_all(ps, s, m)
  return JSON.parse(result)
}
*/
export function projectionInformation(protos: InterfacingSwarms, subscriptions: Subscriptions, role: string, minimize: boolean): DataResult<ProjectionInfo> {
  /* const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const m = minimize.toString()
  const result = JSON.parse(projection_information(ps, s, role, m));
  if (result.type === "ERROR") {
    return result
  } else {
    const data = {...result.data, specialEventTypes: new Set<string>(result.data.specialEventTypes) }
    return {type: "OK", data: data}
  } */
  const result: DataResult<ProjectionInfo> = projection_information(protos, JSON.stringify(subscriptions), role, minimize);
 /*  if (result.type === "ERROR") {
    return result
  } else {
    const data = result.data
    const b = data.branches
    const s = data.specialEventTypes
    const data1 = {...result.data, specialEventTypes: new Set<string>(result.data.specialEventTypes) }
    //return {type: "OK", data: data}
  } */
  return projection_information(protos, JSON.stringify(subscriptions), role, minimize);
}

export function projectionAndInformationNew(protos: InterfacingSwarms, subscriptions: Subscriptions, role: string, machine: Machine, k: number): DataResult<ProjectionInfo> {
/*   const ps = JSON.stringify(protos)
  const s = JSON.stringify(subscriptions)
  const m = JSON.stringify(machine)
  const result = JSON.parse(projection_information_new(ps, s, role, m, k.toString()));
  if (result.type === "ERROR") {
    return result
  } else {
    const data = {...result.data, specialEventTypes: new Set<string>(result.data.specialEventTypes) }
    return {type: "OK", data: data}
  } */
  throw console.error();

}