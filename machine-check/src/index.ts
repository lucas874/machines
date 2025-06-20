import { check_swarm, check_projection, check_wwf_swarm, exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub, check_composed_projection,
  revised_projection, project_combine, compose_protocols, projection_information,
  CheckResult, MachineType, SwarmProtocolType, Subscriptions, InterfacingSwarms as InterfacingSwarmsInner, CompositionComponent as CompositionComponentInner, Role, DataResult, Granularity,
  ProjectionInfo, InterfacingProtocols } from '../pkg/machine_check.js'
export { MachineType, SwarmProtocolType, Subscriptions, Role, CheckResult as Result, DataResult, Granularity, ProjectionInfo, InterfacingProtocols }
export type CompositionComponent = CompositionComponentInner<Role>;
export type InterfacingSwarms = InterfacingSwarmsInner<Role>;

export function checkSwarmProtocol(proto: SwarmProtocolType, subscriptions: Subscriptions): CheckResult {
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
): CheckResult {
  const sw = JSON.stringify(swarm)
  const sub = JSON.stringify(subscriptions)
  const m = JSON.stringify(machine)
  const result = check_projection(sw, sub, role, m)
  return JSON.parse(result)
}

export function checkWWFSwarmProtocol(protos: InterfacingProtocols, subscriptions: Subscriptions): CheckResult {
  return check_wwf_swarm(protos, JSON.stringify(subscriptions))
}

export function exactWWFSubscriptions(protos: InterfacingProtocols, subscriptions: Subscriptions): DataResult<Subscriptions> {
  return exact_weak_well_formed_sub(protos, JSON.stringify(subscriptions));
}

export function overapproxWWFSubscriptions(protos: InterfacingProtocols, subscriptions: Subscriptions, granularity: Granularity): DataResult<Subscriptions> {
  return overapproximated_weak_well_formed_sub(protos, JSON.stringify(subscriptions), granularity);
}

export function checkComposedProjection(
  protos: InterfacingProtocols,
  subscriptions: Subscriptions,
  role: Role,
  machine: MachineType,
): CheckResult {
  return check_composed_projection(protos, JSON.stringify(subscriptions), role, machine)
}

export function revisedProjection(
  proto: SwarmProtocolType,
  subscriptions: Subscriptions,
  role: Role,
  minimize: boolean
): DataResult<MachineType> {
  return revised_projection(proto, JSON.stringify(subscriptions), role, minimize)
}

export function projectCombineMachines(protos: InterfacingProtocols, subscriptions: Subscriptions, role: string, minimize: boolean): DataResult<MachineType> {
  return project_combine(protos, JSON.stringify(subscriptions), role, minimize)
}

export function composeProtocols(protos: InterfacingProtocols): DataResult<SwarmProtocolType> {
  return compose_protocols(protos)
}

export function projectionInformation(protos: InterfacingProtocols, subscriptions: Subscriptions, role: Role, machine: MachineType, k: number, minimize: boolean): DataResult<ProjectionInfo> {
  return projection_information(protos, JSON.stringify(subscriptions), role, machine, k, minimize);
}