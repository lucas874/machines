/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, Result, DataResult, overapproxWFSubscriptions, checkComposedSwarmProtocol, InterfacingProtocols, composeProtocols} from '@actyx/machine-check'
import chalk from "chalk";

export const manifest = {
  appId: 'com.example.warehouse-factory',
  displayName: 'warehouse-factory',
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
    // sent by the assembly robot when a product has been assembled
    export const product = MachineEvent.design('product')
        .withPayload<{productName: string}>()

    // declare a precisely typed tuple of all events we can now choose from
    export const allEvents = [request, bid, selected, deliver, ack, product] as const
}

export const TransportOrder = SwarmProtocol.make('TransportOrder', Events.allEvents)

export const transportOrderProtocol: SwarmProtocolType = {
  initial: 'initial',
  transitions: [
    {source: 'initial', target: 'auction', label: {cmd: 'request', role: 'warehouse', logType: [Events.request.type]}},
    {source: 'auction', target: 'auction', label: {cmd: 'bid', role: 'transportRobot', logType: [Events.bid.type]}},
    {source: 'auction', target: 'delivery', label: {cmd: 'select', role: 'transportRobot', logType: [Events.selected.type]}},
    {source: 'delivery', target: 'delivered', label: {cmd: 'deliver', role: 'transportRobot', logType: [Events.deliver.type]}},
    {source: 'delivered', target: 'acknowledged', label: {cmd: 'acknowledge', role: 'warehouse', logType: [Events.ack.type]}},
  ]
}

export const assemblyLineProtocol: SwarmProtocolType = {
  initial: 'initial',
  transitions: [
    {source: 'initial', target: 'delivery', label: { cmd: 'request', role: 'warehouse', logType: [Events.request.type]}},
    {source: 'delivery', target: 'acknowledge-delivery', label: { cmd: 'acknowledge', role: 'warehouse', logType: [Events.ack.type]}},
    {source: 'acknowledge-delivery', target: 'done', label: { cmd: 'assemble', role: 'assemblyRobot', logType: [Events.product.type] }},
  ]
}


// Well-formed subscription for the warehouse protocol
const result_subs_warehouse: DataResult<Subscriptions>
  = overapproxWFSubscriptions([transportOrderProtocol], {}, 'TwoStep')
if (result_subs_warehouse.type === 'ERROR') throw new Error(result_subs_warehouse.errors.join(', '))
export const subsWarehouse: Subscriptions = result_subs_warehouse.data

// Well-formed subscription for the factory protocol
const resultSubsFactory: DataResult<Subscriptions>
  = overapproxWFSubscriptions([assemblyLineProtocol], {}, 'TwoStep')
if (resultSubsFactory.type === 'ERROR') throw new Error(resultSubsFactory.errors.join(', '))
export var subsFactory: Subscriptions = resultSubsFactory.data

// Well-formed subscription for the warehouse || factory protocol
const resultSubsComposition: DataResult<Subscriptions>
  = overapproxWFSubscriptions([transportOrderProtocol, assemblyLineProtocol], {}, 'TwoStep')
if (resultSubsComposition.type === 'ERROR') throw new Error(resultSubsComposition.errors.join(', '))
export var subscriptions: Subscriptions = resultSubsComposition.data

// check that the subscription generated for the composition is indeed well-formed
const resultCheckWf: Result = checkComposedSwarmProtocol([transportOrderProtocol, assemblyLineProtocol], subscriptions)
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