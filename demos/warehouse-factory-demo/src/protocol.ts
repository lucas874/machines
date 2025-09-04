/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, Result, DataResult, overapproxWFSubscriptions, checkComposedSwarmProtocol, InterfacingProtocols } from '@actyx/machine-check'
import chalk from "chalk";

export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'Car Factory',
  version: '1.0.0',
}

type ClosingTimePayload = { timeOfDay: string }
type PartIDPayload = {partName: string}
type PosPayload = {position: string, partName: string}
type PartPayload = {partName: string}
type CarPayload = {partName: string, modelName: string}

export namespace Events {
  export const partID = MachineEvent.design('partID').withPayload<PartIDPayload>()
  export const part = MachineEvent.design('part').withPayload<PartPayload>()
  export const pos = MachineEvent.design('pos').withPayload<PosPayload>()
  export const time = MachineEvent.design('time').withPayload<ClosingTimePayload>()
  export const car = MachineEvent.design('car').withPayload<CarPayload>()

  export const allEvents = [partID, part, pos, time, car] as const
}

export const Composition = SwarmProtocol.make('Composition', Events.allEvents)

export const Gwarehouse: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {source: '0', target: '1', label: {cmd: 'request', role: 'T', logType: [Events.partID.type]}},
    {source: '1', target: '2', label: {cmd: 'get', role: 'FL', logType: [Events.pos.type]}},
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

export const warehouse_protocol: InterfacingProtocols = [Gwarehouse]
export const factory_protocol: InterfacingProtocols = [Gfactory]
export const warehouse_factory_protocol: InterfacingProtocols = [Gwarehouse, Gfactory]

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

// Well-formed subscription for the warehouse || factory protocol
const result_subs_composition: DataResult<Subscriptions>
  = overapproxWFSubscriptions(warehouse_factory_protocol, {}, 'TwoStep')
if (result_subs_composition.type === 'ERROR') throw new Error(result_subs_composition.errors.join(', '))
export var subs_composition: Subscriptions = result_subs_composition.data

// outcomment the line below to make well-formedness check fail
//subs_composition['FL'] = ['pos']

// check that the subscription generated for the composition is indeed well-formed
const result_check_wf: Result = checkComposedSwarmProtocol(warehouse_factory_protocol, subs_composition)
if (result_check_wf.type === 'ERROR') throw new Error(result_check_wf.errors.join(', \n'))

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}

export const printState = (machineName: string, stateName: string, statePayload: any) => {
  console.log(chalk.bgBlack.white.bold`${machineName} - State: ${stateName}. Payload: ${statePayload ? JSON.stringify(statePayload, null, 0) : "{}"}`)
}