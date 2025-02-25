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


/*
 * Example from CoPLaWS slides by Florian Furbach
 */
export namespace Events {
  export const partID = MachineEvent.design('partID').withPayload<PartIDPayload>()
  export const part = MachineEvent.design('part').withPayload<PartPayload>()
  export const position = MachineEvent.design('position').withPayload<PositionPayload>()
  export const time = MachineEvent.design('time').withPayload<TimePayload>()

  export const allEvents = [partID, part, position, time] as const
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

export const interfacing_swarms: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}]

const result_subs: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarms, {}, 'Medium')
if (result_subs.type === 'ERROR') throw new Error(result_subs.errors.join(', '))
export const subs: Subscriptions = result_subs.data

const result_project_all = projectAll(interfacing_swarms, subs)

if (result_project_all.type === 'ERROR') throw new Error('error getting subscription')
export const all_projections: MachineType[] = result_project_all.data

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}
