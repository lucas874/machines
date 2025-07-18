import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType, Subscriptions, MachineType, checkWWFSwarmProtocol, DataResult, InterfacingSwarms, exactWWFSubscriptions, overapproxWWFSubscriptions, composeProtocols} from '../../..'
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
const interfacing_swarms: InterfacingSwarms = [{protocol: G1, interface: null}, {protocol: G2, interface: 'T'}, {protocol: G3, interface: 'F'}]
const exact_result_subscriptions: DataResult<Subscriptions> = exactWWFSubscriptions(interfacing_swarms, {})
const overapprox_result_subscriptions: DataResult<Subscriptions> = overapproxWWFSubscriptions(interfacing_swarms, {}, "Coarse")

describe('subscriptions', () => {
  it('exact should be ok', () => {
    expect(exact_result_subscriptions.type).toBe('OK')
  })

  it('overapproximation should be ok', () => {
    expect(overapprox_result_subscriptions.type).toBe('OK')
  })
})

if (exact_result_subscriptions.type === 'ERROR') throw new Error('error getting subscription')
const exact_subscriptions: Subscriptions = exact_result_subscriptions.data

if (overapprox_result_subscriptions.type === 'ERROR') throw new Error('error getting subscription')
const overapprox_subscriptions: Subscriptions = overapprox_result_subscriptions.data


describe('checkWWFSwarmProtocol G1 || G2 || G3 with generated subsription', () => {
  it('should be weak-well-formed protocol w.r.t. exact', () => {
    expect(checkWWFSwarmProtocol(interfacing_swarms, exact_subscriptions)).toEqual({
      type: 'OK',
    })
  })

  it('should be weak-well-formed protocol w.r.t. overapproximation', () => {
    expect(checkWWFSwarmProtocol(interfacing_swarms, overapprox_subscriptions)).toEqual({
      type: 'OK',
    })
  })
})

const G2_: SwarmProtocolType = {
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
      label: { cmd: 'build', role: 'F', logType: [Events.car.type] },
    },
  ],
}

const interfacing_swarms_error_1: InterfacingSwarms = [{protocol: G1, interface: null}, {protocol: G2, interface: 'FL'}, {protocol: G3, interface: 'F'}]
const interfacing_swarms_error_2: InterfacingSwarms = [{protocol: G1, interface: null}, {protocol: G2_, interface: 'T'}]
const result_composition_ok = composeProtocols(interfacing_swarms)
const result_composition_error_1 = composeProtocols(interfacing_swarms_error_1)
const result_composition_error_2 = composeProtocols(interfacing_swarms_error_2)

describe('various tests', () => {
  it('should be ok', () => {
    expect(result_composition_ok.type).toBe('OK')
  })

  // fix this error reporting. an empty proto info returned somewhere. keep going instead but propagate errors.
  it('should be not be ok, FL not valid interface', () => {
    expect(result_composition_error_1).toEqual({
        type: 'ERROR',
        errors: [
          "role FL can not be used as interface",
          "event type position does not appear in both protocols",
          "role FL can not be used as interface",
          "event type position does not appear in both protocols",
          "role F can not be used as interface",
          "event type car does not appear in both protocols",
          "role F can not be used as interface",
          "event type car does not appear in both protocols"
        ]
      })
  })
  // change this error reporting as well?
  it('should be not be ok, all events of T not in G2_', () => {
    expect(result_composition_error_2).toEqual({
        type: 'ERROR',
        errors: [
          "event type part does not appear in both protocols",
          "event type part does not appear in both protocols"
        ]
      })
  })
})

// fix this error being recorded twice.
describe('various errors', () => {
  it('subscription for empty list of protocols', () => {
    expect(overapproxWWFSubscriptions([], {}, "Coarse")).toEqual({
      type: 'ERROR',
      errors: [
        "invalid argument",
        "invalid argument"
      ]
    })
  })
  it('subscription for empty list of protocols', () => {
    expect(exactWWFSubscriptions([], {})).toEqual({
      type: 'ERROR',
      errors: [
        "invalid argument",
        "invalid argument"
      ]
    })
  })
})