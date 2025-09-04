/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, Result, DataResult, overapproxWFSubscriptions, checkComposedSwarmProtocol, MachineType, InterfacingProtocols} from '@actyx/machine-check'

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
export const warehouse_protocol: InterfacingProtocols = [Gwarehouse]
export const factory_protocol: InterfacingProtocols = [Gfactory]
export const quality_protocol: InterfacingProtocols = [Gquality]
export const warehouse_factory_quality_protocol: InterfacingProtocols = [Gwarehouse, Gfactory, Gquality]

// Well-formed subscription for the warehouse protocol
const result_subs_warehouse: DataResult<Subscriptions>
  = overapproxWFSubscriptions(warehouse_protocol, {}, 'TwoStep')
if (result_subs_warehouse.type === 'ERROR') throw new Error(result_subs_warehouse.errors.join(', '))
export var subs_warehouse: Subscriptions = result_subs_warehouse.data

// Well-formed subscription for the factory protocol
const result_subs_factory: DataResult<Subscriptions>
  = overapproxWFSubscriptions(factory_protocol, {}, 'TwoStep')
if (result_subs_factory.type === 'ERROR') throw new Error(result_subs_factory.errors.join(', '))
export var subs_factory: Subscriptions = result_subs_factory.data

// Well-formed subscription for the quality protocol
const result_subs_quality: DataResult<Subscriptions>
  = overapproxWFSubscriptions(quality_protocol, {}, 'TwoStep')
if (result_subs_quality.type === 'ERROR') throw new Error(result_subs_quality.errors.join(', '))
export var subs_quality: Subscriptions = result_subs_quality.data

const result_subs_composition: DataResult<Subscriptions>
  = overapproxWFSubscriptions(warehouse_factory_quality_protocol, {}, 'TwoStep')
if (result_subs_composition.type === 'ERROR') throw new Error(result_subs_composition.errors.join(', '))
export var subs_composition: Subscriptions = result_subs_composition.data

// check that the subscription generated for the composition is indeed well-formed
const result_check_wf: Result = checkComposedSwarmProtocol(warehouse_factory_quality_protocol, subs_composition)
if (result_check_wf.type === 'ERROR') throw new Error(result_check_wf.errors.join(', '))

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}

export function print_event(e: any) {
  console.log(`received an event: ${JSON.stringify(e.payload, null, 2)}`)
}