/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, checkWWFSwarmProtocol, ResultData, InterfacingSwarms, overapproxWWFSubscriptions, checkComposedProjection, projectAll, MachineType} from '@actyx/machine-check'


export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'Car Factory',
  version: '1.0.0',
}

type TimePayload = { timeOfDay: string }
/*
 * Example from CoPLaWS slides by Florian Furbach
 */
export namespace Events {
  export const partID = MachineEvent.design('partID').withoutPayload()
  export const part = MachineEvent.design('part').withoutPayload()
  export const position = MachineEvent.design('position').withoutPayload()
  export const time = MachineEvent.design('time').withPayload<TimePayload>()
  export const car = MachineEvent.design('car').withoutPayload()
  export const observing = MachineEvent.design('ok').withoutPayload()
  export const report = MachineEvent.design('report').withoutPayload()

  export const allEvents = [partID, part, position, time, car, observing, report] as const
}

export const Composition = SwarmProtocol.make('Composition', Events.allEvents)

export const Gwarehouse: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: {cmd: 'request', role: 'T', logType: [Events.partID.type]}},
    {source: '1', target: '2', label: {cmd: 'get', role: 'FL', logType: [Events.position.type]}},
    {source: '2', target: '0', label: {cmd: 'deliver', role: 'T', logType: [Events.part.type]}},
    {source: '0', target: '3', label: {cmd: 'close', role: 'D', logType: [Events.time.type]}},
  ]}

export const Gfactory: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: { cmd: 'request', role: 'T', logType: [Events.partID.type]}},
    {source: '1', target: '2', label: { cmd: 'deliver', role: 'T', logType: [Events.part.type]}},
    {source: '2', target: '3', label: { cmd: 'build', role: 'R', logType: [Events.car.type] }},
  ]}

export const Gquality: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: { cmd: 'observe', role: 'QCR', logType: [Events.observing.type]}},
    {source: '1', target: '2', label: { cmd: 'build', role: 'R', logType: [Events.car.type] }},
    {source: '2', target: '3', label: { cmd: 'test', role: 'QCR', logType: [Events.report.type] }},
  ]}

//const protocols: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}, {protocol: Gfactory, interface: 'T'}, {protocol: Gquality, interface: 'R'}]
export const interfacing_swarms: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}, {protocol: Gfactory, interface: 'T'}]
//export const interfacing_swarms: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}]
//export const interfacing_swarms: InterfacingSwarms = [{protocol: Gfactory, interface: null}]
const result_subs: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarms, {}, 'Medium')
if (result_subs.type === 'ERROR') throw new Error(result_subs.errors.join(', '))
export const subs: Subscriptions = result_subs.data


const result_project_all = projectAll(interfacing_swarms, subs)

if (result_project_all.type === 'ERROR') throw new Error('error getting subscription')
export const all_projections: MachineType[] = result_project_all.data