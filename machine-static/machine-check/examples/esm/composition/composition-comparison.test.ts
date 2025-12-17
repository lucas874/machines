import { describe, expect, it } from '@jest/globals'
import { checkComposedSwarmProtocol } from '../../..'
import { SwarmProtocolType, Subscriptions, DataResult, InterfacingProtocols, exactWFSubscriptions, overapproxWFSubscriptions, composeProtocols } from 'machine-core'
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
const interfacing_swarms: InterfacingProtocols = [G1, G2, G3]
const exact_result_subscriptions: DataResult<Subscriptions> = exactWFSubscriptions(interfacing_swarms, {})
const overapprox_result_subscriptions: DataResult<Subscriptions> = overapproxWFSubscriptions(interfacing_swarms, {}, "Coarse")

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
    expect(checkComposedSwarmProtocol(interfacing_swarms, exact_subscriptions)).toEqual({
      type: 'OK',
    })
  })

  it('should be weak-well-formed protocol w.r.t. overapproximation', () => {
    expect(checkComposedSwarmProtocol(interfacing_swarms, overapprox_subscriptions)).toEqual({
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

const interfacing_swarms_error_1: InterfacingProtocols = [G1, G2, G3,]
const interfacing_swarms_error_2: InterfacingProtocols = [G1, G2_,]
const result_composition_ok = composeProtocols(interfacing_swarms)
const result_composition_error_1 = composeProtocols(interfacing_swarms_error_1)
const result_composition_error_2 = composeProtocols(interfacing_swarms_error_2)

describe('various tests', () => {
  it('should be ok', () => {
    expect(result_composition_ok.type).toBe('OK')
  })

  // fix this error reporting. an empty proto info returned somewhere. keep going instead but propagate errors.
  it('should be be ok', () => {
    expect(result_composition_error_1.type).toEqual('OK')
  })
  // change this error reporting as well?
  it('should be not be ok, even though all events of T not in G2_', () => {
    expect(result_composition_error_2.type).toEqual('OK')
  })
})

// fix this error being recorded twice.
describe('various errors', () => {
  it('subscription for empty list of protocols', () => {
    expect(overapproxWFSubscriptions([], {}, "Coarse")).toEqual({
      type: 'OK',
      data: {}
    })
  })
  it('subscription for empty list of protocols', () => {
    expect(exactWFSubscriptions([], {})).toEqual({
      type: 'OK',
      data: {}
    })
  })
})