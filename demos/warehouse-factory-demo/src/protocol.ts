/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, Result, ResultData, InterfacingSwarms, overapproxWWFSubscriptions, checkWWFSwarmProtocol, projectAll, MachineType} from '@actyx/machine-check'

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

export namespace Events {
  export const partReq = MachineEvent.design('partReq').withPayload<PartReqPayload>()
  export const partOK = MachineEvent.design('partOK').withPayload<PartOKPayload>()
  export const pos = MachineEvent.design('pos').withPayload<PosPayload>()
  export const closingTime = MachineEvent.design('closingTime').withPayload<ClosingTimePayload>()
  export const car = MachineEvent.design('car').withPayload<CarPayload>()

  export const allEvents = [partReq, partOK, pos, closingTime, car] as const
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

export const warehouse_protocol: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}]
export const factory_protocol: InterfacingSwarms = [{protocol: Gfactory, interface: null}]
export const warehouse_factory_protocol: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}, {protocol: Gfactory, interface: 'T'}]

// Well-formed subscription for the warehouse protocol
const result_subs_warehouse: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(warehouse_protocol, {}, 'TwoStep')
if (result_subs_warehouse.type === 'ERROR') throw new Error(result_subs_warehouse.errors.join(', '))
export var subs_warehouse: Subscriptions = result_subs_warehouse.data

// Well-formed subscription for the factory protocol
const result_subs_factory: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(factory_protocol, {}, 'TwoStep')
if (result_subs_factory.type === 'ERROR') throw new Error(result_subs_factory.errors.join(', '))
export var subs_factory: Subscriptions = result_subs_factory.data

// Well-formed subscription for the warehouse || factory protocol
const result_subs_composition: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(warehouse_factory_protocol, {}, 'TwoStep')
if (result_subs_composition.type === 'ERROR') throw new Error(result_subs_composition.errors.join(', '))
export var subs_composition: Subscriptions = result_subs_composition.data

// outcomment the line below to make well-formedness check fail
//subs_composition['FL'] = ['pos']

// check that the subscription generated for the composition is indeed well-formed
const result_check_wf: Result = checkWWFSwarmProtocol(warehouse_factory_protocol, subs_composition)
if (result_check_wf.type === 'ERROR') throw new Error(result_check_wf.errors.join(', '))

const result_project_all = projectAll(warehouse_factory_protocol, subs_composition)

if (result_project_all.type === 'ERROR') throw new Error('error getting subscription')
export const all_projections: MachineType[] = result_project_all.data

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}
