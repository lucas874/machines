import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType, checkSwarmProtocol, Subscriptions, checkWWFSwarmProtocol, ResultData, CompositionComponent, InterfacingSwarms, exactWWFSubscriptions, overapproxWWFSubscriptions} from '../../..'
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
const G1_: InterfacingSwarms = [{protocol: G1, interface: null}]
const G2_: InterfacingSwarms = [{protocol: G2, interface: null}]
const G3_: InterfacingSwarms = [{protocol: G3, interface: null}]
const result_subscriptions1: ResultData<Subscriptions> = exactWWFSubscriptions(G1_)
const result_subscriptions2: ResultData<Subscriptions> = exactWWFSubscriptions(G2_)
const result_subscriptions3: ResultData<Subscriptions> = exactWWFSubscriptions(G3_)

describe('extended subscriptions', () => {
  it('subscription1 should be ok', () => {
    expect(result_subscriptions1.type).toBe('OK')
  })

  it('subscription1 should be ok', () => {
    expect(result_subscriptions2.type).toBe('OK')
  })

  it('subscription3 should be ok', () => {
    expect(result_subscriptions3.type).toBe('OK')
  })
})

if (result_subscriptions1.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions1: Subscriptions = result_subscriptions1.data

if (result_subscriptions2.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions2: Subscriptions = result_subscriptions2.data

if (result_subscriptions3.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions3: Subscriptions = result_subscriptions3.data

console.log(subscriptions1)

describe('checkSwarmProtocol G1', () => {
  it('should catch not well-formed protocol', () => {
    expect(checkSwarmProtocol(G1, subscriptions1)).toEqual({
      type: 'ERROR',
      errors: [
        "subsequently involved role D does not subscribe to guard in transition (1)--[get@FL<position>]-->(2)",
        "subsequently involved role FL does not subscribe to guard in transition (2)--[deliver@T<part>]-->(0)"
      ],
    })
  })
})
/*
describe('checkSwarmProtocol G2', () => {
  it('should catch not well-formed protocol', () => {
    expect(checkSwarmProtocol(G2, subscriptions2)).toEqual({
      type: 'ERROR',
      errors: [
        "subsequently involved role F does not subscribe to guard in transition (0)--[request@T<partID>]-->(1)"
      ],
    })
  })
})

describe('checkSwarmProtocol G3', () => {
  it('should catch not well-formed protocol', () => {
    expect(checkSwarmProtocol(G3, subscriptions3)).toEqual({
      type: 'ERROR',
      errors: [
        "subsequently involved role QCR does not subscribe to guard in transition (0)--[build@F<car>]-->(1)"
      ],
    })
  })
}) */

/* describe('checkWWFSwarmProtocol G1', () => {
  it('should be weak-well-formed protocol', () => {
    expect(checkWWFSwarmProtocol(G1_, subscriptions1)).toEqual({
      type: 'OK',
    })
  })
})

describe('checkWWFSwarmProtocol G2', () => {
  it('should be weak-well-formed protocol', () => {
    expect(checkWWFSwarmProtocol(G2_, subscriptions2)).toEqual({
      type: 'OK',
    })
  })
})

describe('checkWWFSwarmProtocol G3', () => {
  it('should be weak-well-formed protocol', () => {
    expect(checkWWFSwarmProtocol(G3_, subscriptions3)).toEqual({
      type: 'OK',
    })
  })
}) */