import {
  CheckResult, MachineType, SwarmProtocolType, Subscriptions, Role, DataResult, Granularity, InterfacingProtocols,
  exact_well_formed_sub, overapproximated_well_formed_sub,
} from '../pkg/machine_types.js'
export { MachineType, SwarmProtocolType, Subscriptions, Role, CheckResult, DataResult }

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