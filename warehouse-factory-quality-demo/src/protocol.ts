/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, ResultData, InterfacingSwarms, overapproxWWFSubscriptions, projectAll, MachineType} from '@actyx/machine-check'

export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'Car Factory',
  version: '1.0.0',
}

type ClosingTimePayload = { timeOfDay: string }
type PartReqPayload = {id: string}
type PosPayload = {position: string, part: string}
type PartOKPayload = {part: string}
type CarPayload = {part: string, modelName: string}
type ReportPayload = {modelName: string, decision: string}

export namespace Events {
  export const partReq = MachineEvent.design('partReq').withPayload<PartReqPayload>()
  export const partOK = MachineEvent.design('partOK').withPayload<PartOKPayload>()
  export const pos = MachineEvent.design('pos').withPayload<PosPayload>()
  export const closingTime = MachineEvent.design('closingTime').withPayload<ClosingTimePayload>()
  export const car = MachineEvent.design('car').withPayload<CarPayload>()
  export const observing = MachineEvent.design('obs').withoutPayload()
  export const report = MachineEvent.design('report').withPayload<ReportPayload>()

  export const allEvents = [partReq, partOK, pos, closingTime, car, observing, report] as const
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

export const Gfactory: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: { cmd: 'request', role: 'T', logType: [Events.partReq.type]}},
    {source: '1', target: '2', label: { cmd: 'deliver', role: 'T', logType: [Events.partOK.type]}},
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
  = overapproxWWFSubscriptions(interfacing_swarms, {}, 'TwoStep')
if (result_subs.type === 'ERROR') throw new Error(result_subs.errors.join(', '))
export const subs: Subscriptions = result_subs.data

const result_subswh: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarmswh, {}, 'TwoStep')
if (result_subswh.type === 'ERROR') throw new Error(result_subswh.errors.join(', '))
export const subswh: Subscriptions = result_subswh.data

const result_subsf: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarmsf, {}, 'TwoStep')
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
