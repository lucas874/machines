import { MachineType, Role, Subscriptions, SwarmProtocolType } from 'machine-core';
import { check_swarm, check_projection, check_composed_swarm, check_composed_projection, InterfacingProtocols, CheckResult } from '../pkg/machine_check.js'
export { CheckResult }

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
  return check_swarm(proto, subscriptions)
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
  return check_projection(swarm, subscriptions, role, machine)
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
  return check_composed_swarm(protos, subscriptions)
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
  return check_composed_projection(protos, subscriptions, role, machine)
}