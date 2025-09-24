import { check_swarm, check_projection, check_composed_swarm, exact_well_formed_sub, overapproximated_well_formed_sub, check_composed_projection,
  revised_projection, project_combine, compose_protocols, projection_information,
  CheckResult, MachineType, SwarmProtocolType, Subscriptions, InterfacingSwarms as InterfacingSwarmsInner, CompositionComponent as CompositionComponentInner, Role, DataResult, Granularity,
  ProjectionInfo, InterfacingProtocols } from '../pkg/machine_check.js'
export { MachineType, SwarmProtocolType, Subscriptions, Role, CheckResult as Result, DataResult, Granularity, ProjectionInfo, InterfacingProtocols }
export type CompositionComponent = CompositionComponentInner<Role>;
export type InterfacingSwarms = InterfacingSwarmsInner<Role>;

/**
 * Check that a swarm protocol is *well-formed* w.r.t. a subscription
 * (using the [Behavioural Types for Local-First Software](https://drops.dagstuhl.de/storage/00lipics/lipics-vol263-ecoop2023/LIPIcs.ECOOP.2023.15/LIPIcs.ECOOP.2023.15.pdf)
 * defninition of well-formedness).
 *
 * @param proto - A swarm protocol.
 * @param subscriptions - A subscription.
 * @returns - Result indicating successful verification or a list of error messages.
 */
export function checkSwarmProtocol(proto: SwarmProtocolType, subscriptions: Subscriptions): CheckResult {
  return check_swarm(proto, JSON.stringify(subscriptions))
}

/**
 * Check that a machine correctly implements some role of a swarm protocol
 * (using the [Behavioural Types for Local-First Software](https://drops.dagstuhl.de/storage/00lipics/lipics-vol263-ecoop2023/LIPIcs.ECOOP.2023.15/LIPIcs.ECOOP.2023.15.pdf)
 * defninition of *projection*).
 *
 * @param swarm - A swarm protocol.
 * @param subscriptions - A subscription.
 * @param role - The role to check against.
 * @param machine - The machine to check.
 * @returns - Result indicating successful verification or a list of errors.
 */
export function checkProjection(
  swarm: SwarmProtocolType,
  subscriptions: Subscriptions,
  role: string,
  machine: MachineType,
): CheckResult {
  return check_projection(swarm, JSON.stringify(subscriptions), role, machine)
}

/**
 * Check that a composed swarm protocol is *well-formed* w.r.t. a subscription.
 * The composition is given implicitly as an array of the swarm protocols that
 * form the composition. A single swarm protocol can be checked for well-formedness
 * by passing an array containing just that single swarm protocol.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @param subscriptions - A subscription.
 * @returns - Result indicating successful verification or a list of error messages.
 */
export function checkComposedSwarmProtocol(protos: InterfacingProtocols, subscriptions: Subscriptions): CheckResult {
  return check_composed_swarm(protos, JSON.stringify(subscriptions))
}

/**
 * Generate the smallest subscription that is well-formed w.r.t. to
 * a swarm protocol composition and contains an input subscription.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @param subscriptions - A subscription.
 * @returns - Result containing the computed subscription or a list of error messages.
 */
export function exactWFSubscriptions(protos: InterfacingProtocols, subscriptions: Subscriptions): DataResult<Subscriptions> {
  return exact_well_formed_sub(protos, JSON.stringify(subscriptions));
}

/**
 * Generate an overapproximation of the smallest subscription that
 * is well-formed w.r.t. to a swarm protocol composition and
 * contains an input subscription.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @param subscriptions - A subscription.
 * @param granularity - The precision of the approximation.
 * @returns - Result containing the computed subscription or a list of error messages.
 */
export function overapproxWFSubscriptions(protos: InterfacingProtocols, subscriptions: Subscriptions, granularity: Granularity): DataResult<Subscriptions> {
  return overapproximated_well_formed_sub(protos, JSON.stringify(subscriptions), granularity);
}

/**
 * Check that a machine correctly implements some role of a (possibly composed) swarm protocol.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @param subscriptions - A subscription.
 * @param role - The role (given as a string).
 * @param machine - The machine to check.
 * @returns - Result indicating successful verification or a list of error messages.
 */
export function checkComposedProjection(
  protos: InterfacingProtocols,
  subscriptions: Subscriptions,
  role: Role,
  machine: MachineType,
): CheckResult {
  return check_composed_projection(protos, JSON.stringify(subscriptions), role, machine)
}

/**
 * Compute the projection of a swarm protocol over a role w.r.t. a subscription.
 *
 * @param proto - A swarm protocol.
 * @param subscriptions - A subscription.
 * @param role - A role (given as a string).
 * @returns - Result containing the projection or a list of error messages.
 */
export function revisedProjection(
  proto: SwarmProtocolType,
  subscriptions: Subscriptions,
  role: Role,
  minimize: boolean
): DataResult<MachineType> {
  return revised_projection(proto, JSON.stringify(subscriptions), role, minimize)
}

/**
 * Compute the projection of a composed swarm protocol over a role w.r.t. a subscription.
 * Computes the projection of each swarm protocol in the composition over the role and
 * combines the results as opposed to expanding the composition and computing the projection.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @param subscriptions - A subscription.
 * @param role - A role (given as a string).
 * @param minimize - The projection is minimized if ```minimize``` is true and returned as is otherwise.
 * @returns - Result containing the projection or a list of error messages.
 */
export function projectCombineMachines(protos: InterfacingProtocols, subscriptions: Subscriptions, role: string, minimize: boolean): DataResult<MachineType> {
  return project_combine(protos, JSON.stringify(subscriptions), role, minimize)
}

/**
 * Construct the composition of a number of swarm protocols.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @returns - Result containing the expanded composition or a list of error messages.
 */
export function composeProtocols(protos: InterfacingProtocols): DataResult<SwarmProtocolType> {
  return compose_protocols(protos)
}

/**
 * Returns a projection of a composed swarm protocol over a role w.r.t. a subscription
 * and information used for running a branch-tracking adapted machine implementing some role.
 * Computes the projection of each swarm protocol in the composition over the role and composes
 * these and the projection given by the ```machine``` argument.
 *
 * @param role - The role
 * @param protos - An array of swarm protocols representing a composition.
 * @param k - The index of the protocol in ```protos``` for which ```machine``` was implemented.
 * @param subscriptions - A subscription.
 * @param machine - The (unadapted) original machine.
 * @param minimize - The projection is minimized if ```minimize``` is true and returned as is otherwise.
 * @returns Result containing the expanded composition or a list of error messages.
 */
export function projectionInformation(role: Role, protos: InterfacingProtocols, k: number, subscriptions: Subscriptions, machine: MachineType, minimize: boolean): DataResult<ProjectionInfo> {
  return projection_information(role, protos, k, JSON.stringify(subscriptions), machine, minimize);
}