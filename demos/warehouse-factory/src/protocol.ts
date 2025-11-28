import { DataResult, overapproxWFSubscriptions, Subscriptions } from "@actyx/machine-check";
import { MachineEvent, SwarmProtocol } from "@actyx/machine-runner";
import chalk from "chalk";
import * as fs from 'fs';

export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'warehouse-factory',
  version: '1.0.0',
}

export const observe = "observe";
export const QualityControl = "QualityControl";
export const testCar = "testCar";
export const carTestedState = "carTestedState";
export const observingState = "observingState";
export const ADAPTED = "adapted";
export const build = "build";
export const finalState = "finalState";
export const buildState = "buildState";
export const carFinishedState = "carFinishedState";
export const car = "car";
export const Robot = "Robot";
export const buildCar = "buildCar";
export const closingTime = "closingTime";
export const Door = "Door";
export const closeDoor = "closeDoor";
export const closedState = "closedState";
export const partOK = "partOK";
export const Transport = "Transport";
export const deliver = "deliver";
export const position = "position";
export const Forklift = "Forklift";
export const get = "get";
export const deliverState = "deliverState";
export const partRequest = "partRequest";
export const request = "request";
export const requestedState = "requestedState";
export const initialState = "initialState";

export type ClosingTimePayload = { timeOfDay: string }
export type PartPayload = { partName: string }
export type PositionPayload = { position: string, partName: string }
export type CarPayload = {partName: string, modelName: string}
export type ReportPayload = {modelName: string, decision: string}

export namespace Events {
  export const partReqEvent = MachineEvent.design(partRequest).withPayload<PartPayload>()
  export const partOKEvent = MachineEvent.design(partOK).withPayload<PartPayload>()
  export const positionEvent = MachineEvent.design(position).withPayload<PositionPayload>()
  export const closingTimeEvent = MachineEvent.design(closingTime).withPayload<ClosingTimePayload>()
  export const carEvent = MachineEvent.design(car).withPayload<CarPayload>()
  export const observingEvent = MachineEvent.design('obs').withoutPayload()
  export const reportEvent = MachineEvent.design('report').withPayload<ReportPayload>()
  export const allEvents = [partReqEvent, partOKEvent, positionEvent, closingTimeEvent, carEvent, observingEvent, reportEvent] as const
}

export const warehouse = {
  initial: initialState,
  transitions: [
      { source: initialState, target: requestedState, label: { cmd: request, role: Transport, logType: [Events.partReqEvent.type]} },
      { source: requestedState, target: deliverState, label: { cmd: get, role: Forklift, logType: [Events.positionEvent.type]} },
      { source: deliverState, target: initialState, label: { cmd: deliver, role: Transport, logType: [Events.partOKEvent.type]} },
      { source: initialState, target: closedState, label: { cmd: closeDoor, role: Door, logType: [Events.closingTimeEvent.type]} }
  ]
};

export const machineRunnerProtoName = "warehouse-factory" as const
export const Protocol = SwarmProtocol.make(machineRunnerProtoName, Events.allEvents)

const resultSubsWarehouse: DataResult<Subscriptions>
  = overapproxWFSubscriptions([warehouse], {}, 'TwoStep')
if (resultSubsWarehouse.type === 'ERROR') throw new Error(resultSubsWarehouse.errors.join(', '))
export const subsWarehouse: Subscriptions = resultSubsWarehouse.data

export const factory = {
  initial: initialState,
  transitions: [
      { source: initialState, target: requestedState, label: { cmd: request, role: Transport, logType: [Events.partReqEvent.type]} },
      { source: requestedState, target: deliverState, label: { cmd: deliver, role: Transport, logType: [Events.partOKEvent.type]} },
      { source: deliverState, target: carFinishedState, label: { cmd: buildCar, role: Robot, logType: [Events.carEvent.type]} }
  ]
};

const resultSubsFactory: DataResult<Subscriptions> = overapproxWFSubscriptions([factory], {}, 'TwoStep')
if (resultSubsFactory.type === 'ERROR') throw new Error(resultSubsFactory.errors.join(', '))
export const subsFactory: Subscriptions = resultSubsFactory.data

export const quality = {
  initial: initialState,
  transitions: [
      { source: initialState, target: observingState, label: { cmd: observe, role: QualityControl, logType: [Events.observingEvent.type]} },
      { source: observingState, target: carFinishedState, label: { cmd: buildCar, role: Robot, logType: [Events.carEvent.type]} },
      { source: carFinishedState, target: carTestedState, label: { cmd: testCar, role: QualityControl, logType: [Events.reportEvent.type]} }
  ]
}

const resultSubsQuality: DataResult<Subscriptions> = overapproxWFSubscriptions([quality], {}, 'TwoStep')
if (resultSubsQuality.type === 'ERROR') throw new Error(resultSubsQuality.errors.join(', '))
export const subsQuality: Subscriptions = resultSubsQuality.data

const resultSubsWarehouseFactory: DataResult<Subscriptions> = overapproxWFSubscriptions([warehouse, factory], {}, 'TwoStep')
if (resultSubsWarehouseFactory.type === 'ERROR') throw new Error(resultSubsWarehouseFactory.errors.join(', '))
export const subsWarehouseFactory: Subscriptions = resultSubsWarehouseFactory.data

const resultSubsWarehouseFactoryQuality: DataResult<Subscriptions>
  = overapproxWFSubscriptions([warehouse, factory, quality], {}, 'TwoStep')
if (resultSubsWarehouseFactoryQuality.type === 'ERROR') throw new Error(resultSubsWarehouseFactoryQuality.errors.join(', '))
export const subsWarehouseFactoryQuality: Subscriptions = resultSubsWarehouseFactoryQuality.data

export const printState = (machineName: string, stateName: string, statePayload: any, commands?: string[]) => {
  console.log(chalk.bgBlack.white.bold`${machineName} - State: ${stateName}. Payload: ${statePayload ? JSON.stringify(statePayload, null, 0) : "{}"}`)
  if (commands) {
    for (const c of commands) {
      console.log(chalk.bgBlack.red.dim`    ${c}!`);
    }
  }
}

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export const getRandomInt = (min: number, max: number): number => {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}


export const throwMachineImplementationErrors = (errors: { type: "ERROR"; errors: string[]; }): void => {
    throw new Error(errors.errors.join(", \n"))
}

export const logToFile = (filename: string, contents: string): void => {
  fs.writeFileSync(filename, contents)
}