/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType, Subscriptions, checkWWFSwarmProtocol, ResultData, InterfacingSwarms, overapproxWWFSubscriptions, checkComposedProjection} from '../../../..'

/*
 * Example from CoPLaWS slides by Florian Furbach
 */

export namespace Events {
  export const partID = MachineEvent.design('partID').withoutPayload()
  export const part = MachineEvent.design('part').withoutPayload()
  export const position = MachineEvent.design('position').withoutPayload()
  export const time = MachineEvent.design('time').withoutPayload()
  export const car = MachineEvent.design('car').withoutPayload()
  export const observing = MachineEvent.design('ok').withoutPayload()
  export const report = MachineEvent.design('report').withoutPayload()

  export const allEvents = [partID, part, position, time, car, observing, report] as const
}

export const Composition = SwarmProtocol.make('Composition', Events.allEvents)

const Gwarehouse: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: {cmd: 'request', role: 'T', logType: [Events.partID.type]}},
    {source: '1', target: '2', label: {cmd: 'get', role: 'FL', logType: [Events.position.type]}},
    {source: '2', target: '0', label: {cmd: 'deliver', role: 'T', logType: [Events.part.type]}},
    {source: '0', target: '3', label: {cmd: 'close', role: 'D', logType: [Events.time.type]}},
  ]}

const Gfactory: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: { cmd: 'request', role: 'T', logType: [Events.partID.type]}},
    {source: '1', target: '2', label: { cmd: 'deliver', role: 'T', logType: [Events.part.type]}},
    {source: '2', target: '3', label: { cmd: 'build', role: 'R', logType: [Events.car.type] }},
  ]}

const Gquality: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: { cmd: 'observe', role: 'QCR', logType: [Events.observing.type]}},
    {source: '1', target: '2', label: { cmd: 'build', role: 'R', logType: [Events.car.type] }},
    {source: '2', target: '3', label: { cmd: 'test', role: 'QCR', logType: [Events.report.type] }},
  ]}

const protocols: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}, {protocol: Gfactory, interface: 'T'}, {protocol: Gquality, interface: 'R'}]

const result_subs: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(protocols, {}, 'Medium')
if (result_subs.type === 'ERROR') throw new Error(result_subs.errors.join(', '))
const subs: Subscriptions = result_subs.data

describe('checkWWFSwarmProtocol for composition with overapproximated wwf subscription', () => {
  it('should be weak-well-formed protocol composition', () => {
    expect(checkWWFSwarmProtocol(protocols, subs)).toEqual({
      type: 'OK',
    })
  })
})

var fail_subs: Subscriptions = {...subs}
fail_subs['D'] = []

describe('checkWWFSwarmProtocol for composition with non-wwf subscription', () => {
  it('should not be weak-well-formed protocol composition', () => {
    expect(checkWWFSwarmProtocol(protocols, fail_subs)).toEqual({
      type: 'ERROR',
      errors: [
        "active role does not subscribe to any of its emitted event types in transition (0 || 0 || 0)--[close@D<time>]-->(3 || 0 || 0)",
        "subsequently active role D does not subscribe to events in transition (2 || 1 || 0)--[deliver@T<part>]-->(0 || 2 || 0)",
        "active role does not subscribe to any of its emitted event types in transition (0 || 2 || 0)--[close@D<time>]-->(3 || 2 || 0)",
        "active role does not subscribe to any of its emitted event types in transition (0 || 2 || 1)--[close@D<time>]-->(3 || 2 || 1)",
        "active role does not subscribe to any of its emitted event types in transition (0 || 3 || 2)--[close@D<time>]-->(3 || 3 || 2)",
        "active role does not subscribe to any of its emitted event types in transition (0 || 3 || 3)--[close@D<time>]-->(3 || 3 || 3)",
        "subsequently active role D does not subscribe to events in transition (2 || 1 || 1)--[deliver@T<part>]-->(0 || 2 || 1)",
        "active role does not subscribe to any of its emitted event types in transition (0 || 0 || 1)--[close@D<time>]-->(3 || 0 || 1)"
      ]
    })
  })
})

export namespace R {
  export const machine = Composition.makeMachine('R')
  // states
  export const s0 = machine.designEmpty('s0').finish()
  export const s1 = machine.designEmpty('s1').finish()
  export const s2 = machine.designEmpty('s2').finish()
  export const s3 = machine.designEmpty('s3').finish()
  export const s4 = machine.designEmpty('s4').finish()
  export const s5 = machine.designEmpty('s5').finish()
  export const s6 = machine.designEmpty('s6').finish()
  export const s7 = machine.designEmpty('s7').finish()
  // s8 and s9 enable commands
  export const s8 = machine
      .designEmpty('s8')
      .command('build', [Events.car], () => [{}])
      .finish()
  export const s9 = machine
      .designEmpty('s9')
      .command('build', [Events.car], () => [{}])
      .finish()
  export const s10 = machine.designEmpty('s10').finish()
  export const s11 = machine.designEmpty('s11').finish()

  // event consumption
  s0.react([Events.partID], s1, () => undefined)
  s0.react([Events.observing], s2, () => undefined)
  s0.react([Events.time], s3, () => undefined)
  s1.react([Events.part], s4, () => undefined)
  s1.react([Events.observing], s5, () => undefined)
  s2.react([Events.partID], s5, () => undefined)
  s2.react([Events.time], s6, () => undefined)
  s3.react([Events.observing], s6, () => undefined)
  s4.react([Events.time], s7, () => undefined)
  s4.react([Events.observing], s8, () => undefined)
  s5.react([Events.part], s8, () => undefined)
  s7.react([Events.observing], s9, () => undefined)
  s8.react([Events.time], s9, () => undefined)
  s8.react([Events.car], s10, () => undefined)
  s9.react([Events.car], s11, () => undefined)
  s10.react([Events.time], s11, () => undefined)
}

describe('check composed projection', () => {
  it('should match R', () => {
    expect(
      checkComposedProjection(
        protocols,
        subs,
        'R',
        R.machine.createJSONForAnalysis(R.s0)
      ),
    ).toEqual({
      type: 'OK',
    })
  })
})