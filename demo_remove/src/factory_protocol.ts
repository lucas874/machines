/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, checkWWFSwarmProtocol, ResultData, InterfacingSwarms, overapproxWWFSubscriptions, checkComposedProjection, projectAll, MachineType, projectCombineMachines} from '@actyx/machine-check'


export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'Car Factory',
  version: '1.0.0',
}

type TimePayload = { timeOfDay: string }
type PartIDPayload = {id: string}
type PositionPayload = {position: string, part: string}
type PartPayload = {part: string}
type CarPayload = {part: string, modelName: string}
/*
 * Example from CoPLaWS slides by Florian Furbach
 */
export namespace Events {
  export const partID = MachineEvent.design('partID').withPayload<PartIDPayload>()
  export const part = MachineEvent.design('part').withPayload<PartPayload>()
  export const position = MachineEvent.design('position').withPayload<PositionPayload>()
  export const time = MachineEvent.design('time').withPayload<TimePayload>()
  export const car = MachineEvent.design('car').withPayload<CarPayload>()
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

export const interfacing_swarms: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}, {protocol: Gfactory, interface: 'T'}, {protocol: Gquality, interface: 'R'}]
//export const interfacing_swarms: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}, {protocol: Gfactory, interface: 'T'}]
export const interfacing_swarmswh: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}]
export const interfacing_swarmsf: InterfacingSwarms = [{protocol: Gfactory, interface: null}]

const result_subs: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarms, {}, 'Medium')
if (result_subs.type === 'ERROR') throw new Error(result_subs.errors.join(', '))
export const subs: Subscriptions = result_subs.data

const result_subswh: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarmswh, {}, 'Medium')
if (result_subswh.type === 'ERROR') throw new Error(result_subswh.errors.join(', '))
export const subswh: Subscriptions = result_subswh.data

const result_subsf: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarmsf, {}, 'Medium')
if (result_subsf.type === 'ERROR') throw new Error(result_subsf.errors.join(', '))
export const subsf: Subscriptions = result_subsf.data

const result_project_all = projectAll(interfacing_swarms, subs)

if (result_project_all.type === 'ERROR') throw new Error('error getting subscription')
export const all_projections: MachineType[] = result_project_all.data

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}
