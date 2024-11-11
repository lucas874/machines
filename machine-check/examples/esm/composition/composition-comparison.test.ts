import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType, checkSwarmProtocol, getWWFSub, checkWWFSwarmProtocol, ResultData, Subscriptions, composeSubs, CompositionInputVec, composeProtocols } from '../../..'
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

const result_subscriptions1: ResultData = getWWFSub(G1)
const result_subscriptions2: ResultData = getWWFSub(G2)
const result_subscriptions3: ResultData = getWWFSub(G3)

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
const subscriptions1: Subscriptions = JSON.parse(result_subscriptions1.data)

if (result_subscriptions2.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions2: Subscriptions = JSON.parse(result_subscriptions2.data)

if (result_subscriptions3.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions3: Subscriptions = JSON.parse(result_subscriptions3.data)

const composition_input: CompositionInputVec = [{protocol: G1, subscription: subscriptions1, interface: null}, {protocol: G2, subscription: subscriptions2, interface: "T"}, {protocol: G3, subscription: subscriptions3, interface: "F"}]

let result_subscriptions = composeSubs(composition_input)
if (result_subscriptions.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions: Subscriptions = JSON.parse(result_subscriptions.data)

const result_composition = composeProtocols(composition_input)
if (result_composition.type === 'ERROR') throw new Error('error getting subscription')
const composition: SwarmProtocolType = JSON.parse(result_composition.data)

describe('checkSwarmProtocol G1 || G2 || G3', () => {
  it('should catch not well-formed protocol', () => {
    expect(checkSwarmProtocol(composition, subscriptions)).toEqual({
      type: 'ERROR',
      errors: [
        "guard event type ok appears in transitions from multiple states",
        "guard event type time appears in transitions from multiple states",
        "guard event type report appears in transitions from multiple states",
        "guard event type car appears in transitions from multiple states",
        "guard event type notOk appears in transitions from multiple states",
        "subsequently involved role D does not subscribe to guard in transition (1 || 1 || 0)--[get@FL<position>]-->(2 || 1 || 0)",
        "subsequently involved role F does not subscribe to guard in transition (1 || 1 || 0)--[get@FL<position>]-->(2 || 1 || 0)",
        "subsequently involved role QCR does not subscribe to guard in transition (1 || 1 || 0)--[get@FL<position>]-->(2 || 1 || 0)",
        "subsequently involved role TR does not subscribe to guard in transition (1 || 1 || 0)--[get@FL<position>]-->(2 || 1 || 0)",
        "subsequently involved role QCR does not subscribe to guard in transition (2 || 1 || 0)--[deliver@T<part>]-->(0 || 2 || 0)",
        "subsequently involved role TR does not subscribe to guard in transition (2 || 1 || 0)--[deliver@T<part>]-->(0 || 2 || 0)",
        "subsequently involved role D does not subscribe to guard in transition (3 || 3 || 1)--[test@TR<report>]-->(3 || 3 || 2)",
        "subsequently involved role F does not subscribe to guard in transition (3 || 3 || 1)--[test@TR<report>]-->(3 || 3 || 2)",
        "subsequently involved role FL does not subscribe to guard in transition (3 || 3 || 1)--[test@TR<report>]-->(3 || 3 || 2)",
        "subsequently involved role T does not subscribe to guard in transition (3 || 3 || 1)--[test@TR<report>]-->(3 || 3 || 2)",
        "subsequently active role D does not subscribe to events in transition (0 || 3 || 1)--[test@TR<report>]-->(0 || 3 || 2)",
        "subsequently involved role QCR subscribes to more events than active role D in transition (0 || 3 || 1)--[test@TR<report>]-->(0 || 3 || 2), namely (report)",
        "subsequently involved role TR subscribes to more events than active role D in transition (0 || 3 || 1)--[test@TR<report>]-->(0 || 3 || 2), namely (report)",
        "subsequently involved role D does not subscribe to guard in transition (0 || 3 || 1)--[test@TR<report>]-->(0 || 3 || 2)",
        "subsequently involved role F does not subscribe to guard in transition (0 || 3 || 1)--[test@TR<report>]-->(0 || 3 || 2)",
        "subsequently involved role FL does not subscribe to guard in transition (0 || 3 || 1)--[test@TR<report>]-->(0 || 3 || 2)",
        "subsequently involved role T does not subscribe to guard in transition (0 || 3 || 1)--[test@TR<report>]-->(0 || 3 || 2)",
      ],
    })
  })
})

// subscription generated using 'implicit' composition is wwf for 'explicit' composition
describe('checkWWFSwarmProtocol G1 || G2 || G3', () => {
  it('should be weak-well-formed protocol', () => {
    expect(checkWWFSwarmProtocol(composition, subscriptions)).toEqual({
      type: 'OK',
    })
  })
})

describe('subscription for empty list of protocols', () => {
  it('should be catch bad input', () => {
    expect(composeSubs([])).toEqual({
      type: 'ERROR',
      errors: [
        "invalid argument"
      ]
    })
  })
})
