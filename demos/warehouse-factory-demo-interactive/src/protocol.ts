/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, Result, DataResult, InterfacingSwarms, overapproxWWFSubscriptions, checkWWFSwarmProtocol, InterfacingProtocols } from '@actyx/machine-check'
import chalk from "chalk";

export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'Car Factory',
  version: '1.0.0',
}

type ClosingTimePayload = { timeOfDay: string }
type PartReqPayload = {partName: string}
type PosPayload = {position: string, partName: string}
type PartOKPayload = {partName: string}
type CarPayload = {partName: string, modelName: string}

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

export const warehouseProtocol: InterfacingProtocols = [Gwarehouse]
export const factoryProtocol: InterfacingProtocols = [Gfactory]
export const warehouseFactoryProtocols: InterfacingProtocols = [Gwarehouse, Gfactory]

// Well-formed subscription for the warehouse protocol
const resultSubsWarehouse: DataResult<Subscriptions>
  = overapproxWWFSubscriptions(warehouseProtocol, {}, 'TwoStep')
if (resultSubsWarehouse.type === 'ERROR') throw new Error(resultSubsWarehouse.errors.join(', '))
export var subsWarehouse: Subscriptions = resultSubsWarehouse.data

// Well-formed subscription for the factory protocol
const resultSubsFactory: DataResult<Subscriptions>
  = overapproxWWFSubscriptions(factoryProtocol, {}, 'TwoStep')
if (resultSubsFactory.type === 'ERROR') throw new Error(resultSubsFactory.errors.join(', '))
export var subsFactory: Subscriptions = resultSubsFactory.data

// Well-formed subscription for the warehouse || factory protocol
const resultSubsComposition: DataResult<Subscriptions>
  = overapproxWWFSubscriptions(warehouseFactoryProtocols, {}, 'TwoStep')
if (resultSubsComposition.type === 'ERROR') throw new Error(resultSubsComposition.errors.join(', '))
export var subsComposition: Subscriptions = resultSubsComposition.data

// outcomment the line below to make well-formedness check fail
//subs_composition['FL'] = ['pos']

// check that the subscription generated for the composition is indeed well-formed
const resultCheckWF: Result = checkWWFSwarmProtocol(warehouseFactoryProtocols, subsComposition)
if (resultCheckWF.type === 'ERROR') throw new Error(resultCheckWF.errors.join(', \n'))

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}

export const printState = (machineName: string, stateName: string, statePayload: any) => {
  console.log(chalk.bgBlack.white.bold`${machineName} - State: ${stateName}. Payload: ${statePayload ? JSON.stringify(statePayload, null, 0) : "{}"}`)
}