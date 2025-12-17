import {
  CheckResult, MachineType, SwarmProtocolType, SubscriptionsWrapped as Subscriptions, Role, DataResult, Granularity, InterfacingProtocols,
  exact_well_formed_sub, overapproximated_well_formed_sub, projection_information, project, compose_protocols,
  ProjectionInfo, ProjToMachineStates
} from '../pkg/machine_core.js'
export { MachineType, SwarmProtocolType, Subscriptions, Role, CheckResult, DataResult, ProjectionInfo, InterfacingProtocols, ProjToMachineStates, Granularity }

/**
 * Generate the smallest subscription that is well-formed w.r.t. to
 * a swarm protocol composition and contains an input subscription.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @param subscriptions - A subscription.
 * @returns - Result containing the computed subscription or a list of error messages.
 */
export function exactWFSubscriptions(protos: InterfacingProtocols, subscriptions: Subscriptions): DataResult<Subscriptions> {
  return exact_well_formed_sub(protos, subscriptions);
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
  return overapproximated_well_formed_sub(protos, subscriptions, granularity);
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
  return projection_information(role, protos, k, subscriptions, machine, minimize);
}

/**
 * Compute the projection of a composed swarm protocol over a role w.r.t. a subscription.
 * Either computes the projection of each swarm protocol in the composition over the role and
 * composes the results or expands the composition and computes the projection of the expanded composition.
 *
 * @param protos - An array of swarm protocols representing a composition.
 * @param subscriptions - A subscription.
 * @param role - A role (given as a string).
 * @param minimize - The projection is minimized if ```minimize``` is true and returned as is otherwise.
 * @param expandProtos - Composition of protocols in ```protos``` is expanded before projection if true, otherwise projection of each swarm protocol is computed and then composed.
 * @returns - Result containing the projection or a list of error messages.
 */
export function projectCombineMachines(protos: InterfacingProtocols, subscriptions: Subscriptions, role: string, minimize: boolean, expandProtos: boolean): DataResult<MachineType> {
  return project(protos, subscriptions, role, minimize, expandProtos)
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