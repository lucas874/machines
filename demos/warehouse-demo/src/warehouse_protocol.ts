/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, checkWWFSwarmProtocol, ResultData, InterfacingSwarms, overapproxWWFSubscriptions, checkComposedProjection, projectAll, MachineType } from '@actyx/machine-check'


export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'Car Factory',
  version: '1.0.0',
}

type ClosingTimePayload = { timeOfDay: string }
type PartReqPayload = {id: string}
type PosPayload = {position: string, part: string}
type PartOKPayload = {part: string}

export namespace Events {
  export const partReq = MachineEvent.design('partReq').withPayload<PartReqPayload>()
  export const partOK = MachineEvent.design('partOK').withPayload<PartOKPayload>()
  export const pos = MachineEvent.design('pos').withPayload<PosPayload>()
  export const closingTime = MachineEvent.design('closingTime').withPayload<ClosingTimePayload>()

  export const allEvents = [partReq, partOK, pos, closingTime] as const
}

export const Composition = SwarmProtocol.make('Composition', Events.allEvents)

export const Gwarehouse: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: {cmd: 'request', role: 'T', logType: [Events.partReq.type]}},
    {source: '1', target: '2', label: {cmd: 'get', role: 'FL', logType: [Events.pos.type]}},
    {source: '2', target: '0', label: {cmd: 'deliver', role: 'T', logType: [Events.partOK.type]}},
    {source: '0', target: '3', label: {cmd: 'close', role: 'D', logType: [Events.closingTime.type]}},
  ]}

export const interfacing_swarms: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}]

const result_subs: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarms, {}, 'TwoStep')
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

export function print_event(e: any) {
  console.log(`received an event: ${JSON.stringify(e.payload, null, 2)}`)
}