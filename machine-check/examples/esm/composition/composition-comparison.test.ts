import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType, Subscriptions, MachineType, checkWWFSwarmProtocol, ResultData, InterfacingSwarms, exactWWFSubscriptions, overapproxWWFSubscriptions, composeProtocols, projectAll} from '../../..'
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
const exact_result_subscriptions: ResultData<Subscriptions> = exactWWFSubscriptions(interfacing_swarms, {})
const overapprox_result_subscriptions: ResultData<Subscriptions> = overapproxWWFSubscriptions(interfacing_swarms, {}, "Coarse")

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
const result_project_all = projectAll(interfacing_swarms, overapprox_subscriptions)

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

describe('all projections', () => {
  it('should be ok and compare with expected', () => {
    expect(result_project_all.type).toBe('OK')

    if (result_project_all.type === 'ERROR') throw new Error('error getting subscription')
    const all_projections: MachineType[] = result_project_all.data
    const expectedFL: MachineType =
      { initial: "{ { 0 } } || { { 0 } } || { { 0 } }",
        transitions:[
          {label:{tag:"Input", eventType:"partID"}, source:"{ { 0 } } || { { 0 } } || { { 0 } }", target: "{ { 1 } } || { { 1 } } || { { 0 } }"},
          {label:{tag:"Input", eventType: "time"}, source: "{ { 0 } } || { { 0 } } || { { 0 } }", target: "{ { 3 }, { 1 }, { 0 } } || { { 0 } } || { { 0 } }"},
          {label:{tag:"Input", eventType:"position"},source:"{ { 1 } } || { { 1 } } || { { 0 } }",target:"{ { 2 } } || { { 1 } } || { { 0 } }"},
          {label:{tag:"Execute",cmd:"get","logType":["position"]},source:"{ { 1 } } || { { 1 } } || { { 0 } }",target:"{ { 1 } } || { { 1 } } || { { 0 } }"},
          {label:{tag:"Input",eventType:"part"},source:"{ { 2 } } || { { 1 } } || { { 0 } }",target:"{ { 0 } } || { { 2 } } || { { 0 } }"},
          {label:{tag:"Input",eventType:"time"},source:"{ { 0 } } || { { 2 } } || { { 0 } }",target:"{ { 3 }, { 1 }, { 0 } } || { { 2 } } || { { 0 } }"},
          {label:{tag:"Input",eventType:"car"},source:"{ { 0 } } || { { 2 } } || { { 0 } }",target:"{ { 0 } } || { { 3 } } || { { 1 } }"},
          {label:{tag:"Input",eventType:"time"},source:"{ { 0 } } || { { 3 } } || { { 1 } }",target:"{ { 3 }, { 1 }, { 0 } } || { { 3 } } || { { 1 } }"},
          {label:{tag:"Input",eventType:"notOk"},source:"{ { 0 } } || { { 3 } } || { { 1 } }",target:"{ { 0 } } || { { 3 } } || { { 3 }, { 3 } }"},
          {label:{tag:"Input",eventType:"ok"},source:"{ { 0 } } || { { 3 } } || { { 1 } }",target:"{ { 0 } } || { { 3 } } || { { 3 }, { 3 } }"},
          {label:{tag:"Input",eventType:"time"},source:"{ { 0 } } || { { 3 } } || { { 3 }, { 3 } }",target:"{ { 3 }, { 1 }, { 0 } } || { { 3 } } || { { 3 }, { 3 } }"},
          {label:{tag:"Input",eventType:"notOk"},source:"{ { 3 }, { 1 }, { 0 } } || { { 3 } } || { { 1 } }",target:"{ { 3 }, { 1 }, { 0 } } || { { 3 } } || { { 3 }, { 3 } }"},
          {label:{tag:"Input",eventType:"ok"},source:"{ { 3 }, { 1 }, { 0 } } || { { 3 } } || { { 1 } }",target:"{ { 3 }, { 1 }, { 0 } } || { { 3 } } || { { 3 }, { 3 } }"},
          {label:{tag:"Input",eventType:"car"},source:"{ { 3 }, { 1 }, { 0 } } || { { 2 } } || { { 0 } }",target:"{ { 3 }, { 1 }, { 0 } } || { { 3 } } || { { 1 } }"}]
      }
    const expectedFLString = JSON.stringify(expectedFL)
    const FLstring = JSON.stringify(all_projections[2])
    expect(FLstring).toBe(expectedFLString)
  })
})