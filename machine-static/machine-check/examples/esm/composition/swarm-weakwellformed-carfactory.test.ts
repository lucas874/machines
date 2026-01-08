import { describe, expect, it } from '@jest/globals'
import { checkSwarmProtocol, checkComposedSwarmProtocol } from '../../..'
import { SwarmProtocolType, Subscriptions, DataResult, InterfacingProtocols, exactWFSubscriptions, overapproxWFSubscriptions, Granularity} from 'machine-core'

import { Events } from './car-factory-protos.js'

/*
 * example from CoPLaWS slides by Florian Furbach
 * protocols are wwf but not wf under the generated subscriptions.
 *
 */

const G1: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {
      source: '0',
      target: '1',
      label: { cmd: 'request', role: 'T', logType: [Events.partID.type] },
    },
    {
      source: '1',
      target: '2',
      label: { cmd: 'get', role: 'FL', logType: [Events.position.type] },
    },
    {
      source: '2',
      target: '0',
      label: { cmd: 'deliver', role: 'T', logType: [Events.part.type] },
    },
    {
      source: '0',
      target: '3',
      label: { cmd: 'close', role: 'D', logType: [Events.time.type] },
    },
  ],
}

const G2: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {
      source: '0',
      target: '1',
      label: { cmd: 'request', role: 'T', logType: [Events.partID.type] },
    },
    {
      source: '1',
      target: '2',
      label: { cmd: 'deliver', role: 'T', logType: [Events.part.type] },
    },
    {
      source: '2',
      target: '3',
      label: { cmd: 'build', role: 'F', logType: [Events.car.type] },
    },
  ],
}

const G3: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {
      source: '0',
      target: '1',
      label: { cmd: 'build', role: 'F', logType: [Events.car.type] },
    },
    {
      source: '1',
      target: '2',
      label: { cmd: 'test', role: 'TR', logType: [Events.report.type] },
    },
    {
      source: '2',
      target: '3',
      label: { cmd: 'accept', role: 'QCR', logType: [Events.ok.type] },
    },
    {
      source: '2',
      target: '3',
      label: { cmd: 'reject', role: 'QCR', logType: [Events.notOk.type] },
    },
  ],
}
const G1_: InterfacingProtocols = [G1]
const G2_: InterfacingProtocols = [G2]
const G3_: InterfacingProtocols = [G3]
const exact_result_subscriptions1: DataResult<Subscriptions> = exactWFSubscriptions(G1_, {})
const exact_result_subscriptions2: DataResult<Subscriptions> = exactWFSubscriptions(G2_, {})
const exact_result_subscriptions3: DataResult<Subscriptions> = exactWFSubscriptions(G3_, {})

describe('extended subscriptions', () => {
  it('subscription1 should be ok', () => {
    expect(exact_result_subscriptions1.type).toBe('OK')
  })

   it('subscription1 should be ok', () => {
    expect(exact_result_subscriptions2.type).toBe('OK')
  })

  it('subscription3 should be ok', () => {
    expect(exact_result_subscriptions3.type).toBe('OK')
  })
})

if (exact_result_subscriptions1.type === 'ERROR') throw new Error('error getting subscription')
const exact_subscriptions1: Subscriptions = exact_result_subscriptions1.data

if (exact_result_subscriptions2.type === 'ERROR') throw new Error('error getting subscription')
const exact_subscriptions2: Subscriptions = exact_result_subscriptions2.data

if (exact_result_subscriptions3.type === 'ERROR') throw new Error('error getting subscription')
const exact_subscriptions3: Subscriptions = exact_result_subscriptions3.data

describe('checkSwarmProtocol for protocols with exact wwf subscription', () => {
  it('should catch not well-formed protocol G1', () => {
    expect(checkSwarmProtocol(G1, exact_subscriptions1)).toEqual({
      type: 'ERROR',
      errors: [
        "subsequently involved role D does not subscribe to guard in transition (1)--[get@FL<position>]-->(2)",
        "subsequently involved role FL does not subscribe to guard in transition (2)--[deliver@T<part>]-->(0)"
      ],
    })
  })

  it('should catch not well-formed protocol G2', () => {
    expect(checkSwarmProtocol(G2, exact_subscriptions2)).toEqual({
      type: 'ERROR',
      errors: [
        "subsequently involved role F does not subscribe to guard in transition (0)--[request@T<partID>]-->(1)"
      ],
    })
  })

  it('should catch not well-formed protocol G3', () => {
    expect(checkSwarmProtocol(G3, exact_subscriptions3)).toEqual({
      type: 'ERROR',
      errors: [
        "subsequently involved role QCR does not subscribe to guard in transition (0)--[build@F<car>]-->(1)"
      ],
    })
  })
})

describe('checkWWFSwarmProtocol for protocols with exact wwf subscription', () => {
  it('should be weak-well-formed protocol G1', () => {
    expect(checkComposedSwarmProtocol(G1_, exact_subscriptions1)).toEqual({
      type: 'OK',
    })
  })

  it('should be weak-well-formed protocol G1', () => {
    expect(checkComposedSwarmProtocol(G2_, exact_subscriptions2)).toEqual({
      type: 'OK',
    })
  })

  it('should be weak-well-formed protocol G1', () => {
    expect(checkComposedSwarmProtocol(G3_, exact_subscriptions3)).toEqual({
      type: 'OK',
    })
  })
})

const granularity: Granularity = "TwoStep"
const overapprox_result_subscriptions1: DataResult<Subscriptions> = overapproxWFSubscriptions(G1_, {}, granularity)
const overapprox_result_subscriptions2: DataResult<Subscriptions> = overapproxWFSubscriptions(G2_, {}, granularity)
const overapprox_result_subscriptions3: DataResult<Subscriptions> = overapproxWFSubscriptions(G3_, {}, granularity)
if (overapprox_result_subscriptions1.type === 'ERROR') throw new Error('error getting subscription')
const overapprox_subscriptions1: Subscriptions = overapprox_result_subscriptions1.data

if (overapprox_result_subscriptions2.type === 'ERROR') throw new Error('error getting subscription')
const overapprox_subscriptions2: Subscriptions = overapprox_result_subscriptions2.data

if (overapprox_result_subscriptions3.type === 'ERROR') throw new Error('error getting subscription')
const overapprox_subscriptions3: Subscriptions = overapprox_result_subscriptions3.data

describe('checkWWFSwarmProtocol for protocols with overapproximated wwf subscription', () => {
  it('should be weak-well-formed protocol G1', () => {
    expect(checkComposedSwarmProtocol(G1_, overapprox_subscriptions1)).toEqual({
      type: 'OK',
    })
  })

  it('should be weak-well-formed protocol G1', () => {
    expect(checkComposedSwarmProtocol(G2_, overapprox_subscriptions2)).toEqual({
      type: 'OK',
    })
  })

  it('should be weak-well-formed protocol G1', () => {
    expect(checkComposedSwarmProtocol(G3_, overapprox_subscriptions3)).toEqual({
      type: 'OK',
    })
  })
})


const Gcomposed: InterfacingProtocols = [G1, G2, G3]
const overapprox_result_subscriptions_composed: DataResult<Subscriptions> = overapproxWFSubscriptions(Gcomposed, {}, "Fine")
if (overapprox_result_subscriptions_composed.type === 'ERROR') throw new Error('error getting subscription')
const overapprox_subscriptions_composed: Subscriptions = overapprox_result_subscriptions_composed.data

describe('checkWWFSwarmProtocol for composition with overapproximated wwf subscription', () => {
  it('should be weak-well-formed protocol G1', () => {
    expect(checkComposedSwarmProtocol(Gcomposed, overapprox_subscriptions_composed)).toEqual({
      type: 'OK',
    })
  })
})