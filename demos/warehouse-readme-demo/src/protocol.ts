/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, Result, DataResult, overapproxWFSubscriptions, checkComposedSwarmProtocol, InterfacingProtocols, composeProtocols} from '@actyx/machine-check'
import chalk from "chalk";

export const manifest = {
  appId: 'com.example.warehouse-factory',
  displayName: 'Warehouse-Factory',
  version: '1.0.0',
}

export namespace Events {
    // sent by the warehouse to get things started
    export const request = MachineEvent.design('request')
        .withPayload<{ id: string; from: string; to: string }>()
    // sent by each available candidate transport robot to register interest
    export const bid = MachineEvent.design('bid')
        .withPayload<{ robot: string; delay: number, id: string }>()
    // sent by the transport robots
    export const selected = MachineEvent.design('selected')
        .withPayload<{ winner: string, id: string }>()
    // sent by the transport robot performing the delivery
    export const deliver = MachineEvent.design('deliver')
        .withPayload<{ id: string }>()
    // sent by the warehouse to acknowledge delivery
    export const ack = MachineEvent.design('acknowledge')
        .withPayload<{ id: string }>()
    // sent by the robot when a car has been built
    export const car = MachineEvent.design('car')
        .withoutPayload()

    export const allEvents = [request, bid, selected, deliver, ack, car] as const
}

export const Composition = SwarmProtocol.make('Composition', Events.allEvents)

export const warehouseProtocol: SwarmProtocolType = {
  initial: 'initial',
  transitions: [
    {source: 'initial', target: 'auction', label: {cmd: 'request', role: 'W', logType: [Events.request.type]}},
    {source: 'auction', target: 'auction', label: {cmd: 'bid', role: 'T', logType: [Events.bid.type]}},
    {source: 'auction', target: 'delivery', label: {cmd: 'select', role: 'T', logType: [Events.selected.type]}},
    {source: 'delivery', target: 'delivered', label: {cmd: 'deliver', role: 'T', logType: [Events.deliver.type]}},
    {source: 'delivered', target: 'acknowledged', label: {cmd: 'acknowledge', role: 'W', logType: [Events.ack.type]}},
  ]}

export const factoryProtocol: SwarmProtocolType = {
  initial: 'initial',
  transitions: [
    {source: 'initial', target: 'req', label: { cmd: 'request', role: 'W', logType: [Events.request.type]}},
    {source: 'req', target: 'ok', label: { cmd: 'acknowledge', role: 'W', logType: [Events.ack.type]}},
    {source: 'ok', target: 'done', label: { cmd: 'build', role: 'R', logType: [Events.car.type] }},
  ]}


// Well-formed subscription for the warehouse protocol
const result_subs_warehouse: DataResult<Subscriptions>
  = overapproxWFSubscriptions([warehouseProtocol], {}, 'TwoStep')
if (result_subs_warehouse.type === 'ERROR') throw new Error(result_subs_warehouse.errors.join(', '))
export const subsWarehouse: Subscriptions = result_subs_warehouse.data

// Well-formed subscription for the factory protocol
const resultSubsFactory: DataResult<Subscriptions>
  = overapproxWFSubscriptions([factoryProtocol], {}, 'TwoStep')
if (resultSubsFactory.type === 'ERROR') throw new Error(resultSubsFactory.errors.join(', '))
export var subsFactory: Subscriptions = resultSubsFactory.data

// Well-formed subscription for the warehouse || factory protocol
const resultSubsComposition: DataResult<Subscriptions>
  = overapproxWFSubscriptions([warehouseProtocol, factoryProtocol], {}, 'TwoStep')
if (resultSubsComposition.type === 'ERROR') throw new Error(resultSubsComposition.errors.join(', '))
export var subscriptions: Subscriptions = resultSubsComposition.data

// check that the subscription generated for the composition is indeed well-formed
const resultCheckWf: Result = checkComposedSwarmProtocol([warehouseProtocol, factoryProtocol], subscriptions)
if (resultCheckWf.type === 'ERROR') throw new Error(resultCheckWf.errors.join(', \n'))

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}

export const printState = (machineName: string, stateName: string, statePayload: any) => {
  console.log(chalk.bgBlack.white.bold`${machineName} - State: ${stateName}. Payload: ${statePayload ? JSON.stringify(statePayload, null, 0) : "{}"}`)
}

/* const thing = composeProtocols([warehouseProtocol, factoryProtocol])
if (thing.type === 'OK') {
    console.log(JSON.stringify(thing.data, null, 2))
} */