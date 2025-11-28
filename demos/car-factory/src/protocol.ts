/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { type SwarmProtocolType, type Subscriptions, type Result, type DataResult, overapproxWFSubscriptions, checkComposedSwarmProtocol, type InterfacingProtocols, composeProtocols} from '@actyx/machine-check'
import chalk from "chalk";
import yargs from 'yargs'
import { hideBin } from 'yargs/helpers'
import { type AppManifest } from '@actyx/sdk';
export const manifest = {
  appId: 'com.example.car-factory',
  displayName: 'car-factory',
  version: '1.0.0',
}
// ctrl + shift + l :DDDD
export type SteelPartsPayload = { part: string }
export type PartialCarBodyPayload = { parts: string[] }
export type CarBodyPayload = { shape: string }
export type PaintedBodyPayload = { shape: string, color: string }
export type ItemDeliveryPayload = { item: string, to: string }
export type BidPayload = { transportId: string, delay: number }
export type SelectedPayload = { winnerTransport: string }
export type GuidanceRequestPayload = { item: string, to: string }
export type RoutePayload = { directions: string[] }
export type EngineInstallationPayload = { shape: string, color: string, engine: string }
export type WheelInstallationPayload = { shape: string, color: string, engine: string, numWheels: number }
export type WindowInstallationPayload = { shape: string, color: string, engine: string, numWindows: number }
export type FinishedCarPayload = { shape: string, color: string, engine: string, numWheels: number, numWindows: number, isOk: boolean }

export const NUMBER_OF_CAR_PARTS = 3

export namespace Events {
  export const steelRoll = MachineEvent.design('SteelRoll').withoutPayload()
  export const steelParts = MachineEvent.design('SteelParts').withPayload<SteelPartsPayload>()
  export const partialCarBody = MachineEvent.design('PartialCarBody').withPayload<PartialCarBodyPayload>()
  export const carBody = MachineEvent.design('CarBody').withPayload<CarBodyPayload>()
  export const paintedCarBody = MachineEvent.design('PaintedCarBody').withPayload<PaintedBodyPayload>()
  export const itemRequest = MachineEvent.design('ItemRequest').withPayload<ItemDeliveryPayload>()
  export const bid = MachineEvent.design('Bid').withPayload<BidPayload>()
  export const selected = MachineEvent.design('Selected').withPayload<SelectedPayload>()
  export const requestGuidance = MachineEvent.design('ReqGuidance').withPayload<GuidanceRequestPayload>()
  export const giveGuidance = MachineEvent.design('GiveGuidance').withPayload<RoutePayload>()
  export const itemPickupBasic = MachineEvent.design('ItemPickupBasic').withPayload<ItemDeliveryPayload>()
  export const itemPickupSmart = MachineEvent.design('ItemPickupSmart').withPayload<ItemDeliveryPayload>()
  export const handover = MachineEvent.design('Handover').withPayload<ItemDeliveryPayload>()
  export const itemDelivery = MachineEvent.design("ItemDeliver").withPayload<ItemDeliveryPayload>()
  export const requestEngine = MachineEvent.design("RequestEngine").withPayload<ItemDeliveryPayload>()
  export const engineInstalled = MachineEvent.design("EngineInstalled").withPayload<EngineInstallationPayload>()
  export const engineChecked = MachineEvent.design("EngineChecked").withPayload<EngineInstallationPayload>()
  export const wheelPickup = MachineEvent.design("WheelPickup").withPayload<WheelInstallationPayload>()
  export const wheelInstalled = MachineEvent.design("WheelInstalled").withPayload<WheelInstallationPayload>()
  export const wheelsDone = MachineEvent.design("AllWheelsInstalled").withPayload<WheelInstallationPayload>()
  export const windowPickup = MachineEvent.design("WindowPickup").withPayload<WindowInstallationPayload>()
  export const windowInstalled = MachineEvent.design("WindowInstalled").withPayload<WindowInstallationPayload>()
  export const windowsDone = MachineEvent.design("AllWindowsInstalled").withPayload<WindowInstallationPayload>()
  export const finishedCar = MachineEvent.design("FinishedCar").withPayload<FinishedCarPayload>()

  export const allEvents =
    [
      steelRoll, steelParts, partialCarBody, carBody,
      paintedCarBody,
      itemRequest, bid, selected, requestGuidance, giveGuidance, itemPickupBasic, itemPickupSmart, handover, itemDelivery,
      requestEngine, engineInstalled, engineChecked,
      wheelPickup, wheelInstalled, wheelsDone,
      windowPickup, windowInstalled, windowsDone,
      finishedCar
    ] as const
}

export const Composition = SwarmProtocol.make('Composition', Events.allEvents)

export namespace SteelPressProtocol {
  const initial = "0"
  const steelPickedUp = "1"
  const steelPressed = "2"
  const bodyAssembled = "3"
  export const stampRole = "Stamp"
  export const bodyAssemblerRole = "BodyAssembler"
  export const steelTransportRole = "SteelTransport"
  export const carBodyCheckerRole = "CarBodyChecker"
  export const cmdPickUpSteel = "pickUpSteelRoll"
  export const cmdPressSteel = "pressSteel"
  export const cmdAssembleBody = "assembleBody"
  export const cmdCheckCarBody = "carBodyDone"

  export const protocol: SwarmProtocolType = {
    initial: initial,
    transitions: [
      {source: initial, target: steelPickedUp, label: {cmd: cmdPickUpSteel, role: steelTransportRole, logType: [Events.steelRoll.type]}},
      {source: steelPickedUp, target: steelPressed, label: {cmd: cmdPressSteel, role: stampRole, logType: [Events.steelParts.type]}},
      {source: steelPressed, target: initial, label: {cmd: cmdAssembleBody, role: bodyAssemblerRole, logType: [Events.partialCarBody.type]}},
      {source: initial, target: bodyAssembled, label: {cmd: cmdCheckCarBody, role: carBodyCheckerRole, logType: [Events.carBody.type]}}
    ]
  }

  const subscriptionsResult: DataResult<Subscriptions>
    = overapproxWFSubscriptions([protocol], {}, 'TwoStep')
  if (subscriptionsResult.type === 'ERROR') throw new Error(subscriptionsResult.errors.join(', '))
  export const subscriptions: Subscriptions = subscriptionsResult.data
}

export namespace PaintShopProtocol {
  const initial = "0"
  const bodyAssembled = "1"
  const bodyPainted = "2"
  export const carBodyCheckerRole = "CarBodyChecker"
  export const painterRole = "Painter"
  export const cmdCheckCarBody = "carBodyDone"
  export const cmdPaintBody = "applyPaint"

  export const protocol: SwarmProtocolType = {
    initial: initial,
    transitions: [
      {source: initial, target: bodyAssembled, label: {cmd: cmdCheckCarBody, role: carBodyCheckerRole, logType: [Events.carBody.type]}},
      {source: bodyAssembled, target: bodyPainted, label: {cmd: cmdPaintBody, role: painterRole, logType: [Events.paintedCarBody.type]}},
    ]
  }

  const subscriptionsResult: DataResult<Subscriptions>
    = overapproxWFSubscriptions([protocol], {}, 'TwoStep')
  if (subscriptionsResult.type === 'ERROR') throw new Error(subscriptionsResult.errors.join(', '))
  export const subscriptions: Subscriptions = subscriptionsResult.data
}

export namespace WarehouseProtocol {
  const initial = "0"
  const itemRequested = "1"
  const transporterSelected = "2"
  const guidanceRequested = "3"
  const guidanceGiven = "4"
  const itemPickedUp = "5"
  const itemHandedOver = "6"
  export const warehouseRole = "Warehouse"
  export const transportRole = "Transport"
  export const baseStationRole = "BaseStation"
  export const cmdRequest = "request"
  export const cmdBid = "bid"
  export const cmdSelect = "select"
  export const cmdNeedGuidance = "needGuidance"
  export const cmdGiveGuidance = "giveGuidance"
  export const cmdBasicPickup = "basicPickup"
  export const cmdSmartPickup = "smartPickup"
  export const cmdHandover = "handover"
  export const cmdDeliver = "deliver"

  export const protocol: SwarmProtocolType = {
    initial: initial,
    transitions: [
      {source: initial, target: itemRequested, label: {cmd: cmdRequest, role: warehouseRole, logType: [Events.itemRequest.type]}},
      {source: itemRequested, target: itemRequested, label: {cmd: cmdBid, role: transportRole, logType: [Events.bid.type]}},
      {source: itemRequested, target: transporterSelected, label: {cmd: cmdSelect, role: transportRole, logType: [Events.selected.type]}},
      {source: transporterSelected, target: guidanceRequested, label: {cmd: cmdNeedGuidance, role: transportRole, logType: [Events.requestGuidance.type]}},
      {source: guidanceRequested, target: guidanceGiven, label: {cmd: cmdGiveGuidance, role: baseStationRole, logType: [Events.giveGuidance.type]}},
      {source: guidanceGiven, target: itemPickedUp, label: {cmd: cmdBasicPickup, role: transportRole, logType: [Events.itemPickupBasic.type]}},
      {source: transporterSelected, target: itemPickedUp, label: {cmd: cmdSmartPickup, role: transportRole, logType: [Events.itemPickupSmart.type]}},
      {source: itemPickedUp, target: itemHandedOver, label: {cmd: cmdHandover, role: transportRole, logType: [Events.handover.type]}},
      {source: itemHandedOver, target: initial, label: {cmd: cmdDeliver, role: warehouseRole, logType: [Events.itemDelivery.type]}}
    ]
  }

  const subscriptionsResult: DataResult<Subscriptions>
    = overapproxWFSubscriptions([protocol], {}, 'TwoStep')
  if (subscriptionsResult.type === 'ERROR') throw new Error(subscriptionsResult.errors.join(', '))
  export const subscriptions: Subscriptions = subscriptionsResult.data
}

export namespace EngineInstallationProtocol {
  const initial = "0"
  const bodyPainted = "1"
  const engineRequested = "2"
  const warehouseEngaged = "3"
  const delivered = "4"
  const engineInstalled = "5"
  const engineChecked = "6"
  export const painterRole = "Painter"
  export const engineInstallerRole = "EngineInstaller"
  export const warehouseRole = "Warehouse"
  export const engineCheckerRole = "EngineChecker"
  export const cmdPaintBody = "applyPaint"
  export const cmdRequestEngine = "requestEngine"
  export const cmdRequest = "request"
  export const cmdDeliver = "deliver"
  export const cmdInstallEngine = "installEngine"
  export const cmdCheckEngine = "checkEngine"

  export const protocol: SwarmProtocolType = {
    initial: initial,
    transitions: [
      {source: initial, target: bodyPainted, label: {cmd: cmdPaintBody, role: painterRole, logType: [Events.paintedCarBody.type]}},
      {source: bodyPainted, target: engineRequested, label: {cmd: cmdRequestEngine, role: engineInstallerRole, logType: [Events.requestEngine.type]}},
      {source: engineRequested, target: warehouseEngaged, label: {cmd: cmdRequest, role: warehouseRole, logType: [Events.itemRequest.type]}},
      {source: warehouseEngaged, target: delivered, label: {cmd: cmdDeliver, role: warehouseRole, logType: [Events.itemDelivery.type]}},
      {source: delivered, target: engineInstalled, label: {cmd: cmdInstallEngine, role: engineInstallerRole, logType: [Events.engineInstalled.type]}},
      {source: engineInstalled, target: engineChecked, label: {cmd: cmdCheckEngine, role: engineCheckerRole, logType: [Events.engineChecked.type]}}
    ]
  }

  const subscriptionsResult: DataResult<Subscriptions>
    = overapproxWFSubscriptions([protocol], {}, 'TwoStep')
  if (subscriptionsResult.type === 'ERROR') throw new Error(subscriptionsResult.errors.join(', '))
  export const subscriptions: Subscriptions = subscriptionsResult.data
}

export namespace WheelInstallationProtocol {
  const initial = "0"
  const pickUpWheel = "1"
  const installWheel = "2"
  const allWheelsInstalled = "3"
  const carChecked = "4"
  export const engineCheckerRole = "EngineChecker"
  export const wheelInstallerRole = "WheelInstaller"
  export const wheelCheckerRole = "WheelChecker"
  export const cmdCheckEngine = "checkEngine"
  export const cmdInstallEngine = "installEngine"
  export const cmdPickUpWheel = "pickUpWheel"
  export const cmdInstallWheel = "installWheel"
  export const cmdWheelsDone = "wheelsDone"
  //export const cmdCheckWheels = "checkWheels"
  export const cmdCheckCar = "checkCar"
  export const qualityControlRole = "QualityControl"

  export const protocol: SwarmProtocolType = {
    initial: initial,
    transitions: [
      {source: initial, target: pickUpWheel, label: {cmd: cmdCheckEngine, role: engineCheckerRole, logType: [Events.engineChecked.type]}},
      {source: pickUpWheel, target: installWheel, label: {cmd: cmdPickUpWheel, role: wheelInstallerRole, logType: [Events.wheelPickup.type]}},
      {source: installWheel, target: pickUpWheel, label: {cmd: cmdInstallWheel, role: wheelInstallerRole, logType: [Events.wheelInstalled.type]}},
      {source: pickUpWheel, target: allWheelsInstalled, label: {cmd: cmdWheelsDone, role: wheelCheckerRole, logType: [Events.wheelsDone.type]}},
      {source: allWheelsInstalled, target: carChecked, label: {cmd: cmdCheckCar, role: qualityControlRole, logType: [Events.finishedCar.type]}},
    ]
  }


  const subscriptionsResult: DataResult<Subscriptions>
    = overapproxWFSubscriptions([protocol], {}, 'TwoStep')
  if (subscriptionsResult.type === 'ERROR') throw new Error(subscriptionsResult.errors.join(', '))
  export const subscriptions: Subscriptions = subscriptionsResult.data
}

export namespace WindowInstallationProtocol {
  const initial = "0"
  const pickUpwindow = "1"
  const installwindow = "2"
  const allwindowsInstalled = "3"
  const carChecked = "4"
  export const engineCheckerRole = "EngineChecker"
  export const windowInstallerRole = "WindowInstaller"
  export const windowCheckerRole = "WindowChecker"
  export const cmdCheckEngine = "checkEngine"
  export const cmdInstallEngine = "installEngine"
  export const cmdPickUpWindow = "pickUpWindow"
  export const cmdInstallwindow = "installWindow"
  export const cmdWindowsDone = "windowsDone"
  export const cmdCheckWindows = "checkWindows"
  export const cmdCheckCar = "checkCar"
  export const qualityControlRole = "QualityControl"

  export const protocol: SwarmProtocolType = {
    initial: initial,
    transitions: [
      {source: initial, target: pickUpwindow, label: {cmd: cmdCheckEngine, role: engineCheckerRole, logType: [Events.engineChecked.type]}},
      {source: pickUpwindow, target: installwindow, label: {cmd: cmdPickUpWindow, role: windowInstallerRole, logType: [Events.windowPickup.type]}},
      {source: installwindow, target: pickUpwindow, label: {cmd: cmdInstallwindow, role: windowInstallerRole, logType: [Events.windowInstalled.type]}},
      {source: pickUpwindow, target: allwindowsInstalled, label: {cmd: cmdWindowsDone, role: windowCheckerRole, logType: [Events.windowsDone.type]}},
      {source: allwindowsInstalled, target: carChecked, label: {cmd: cmdCheckCar, role: qualityControlRole, logType: [Events.finishedCar.type]}}
      //{source: allwindowsInstalled, target: windowsChecked, label: {cmd: cmdCheckWindows, role: windowCheckerRole, logType: [Events.windowsChecked.type]}},
    ]
  }


  const subscriptionsResult: DataResult<Subscriptions>
    = overapproxWFSubscriptions([protocol], {}, 'TwoStep')
  if (subscriptionsResult.type === 'ERROR') throw new Error(subscriptionsResult.errors.join(', '))
  export const subscriptions: Subscriptions = subscriptionsResult.data
}

export namespace QualityControlProtocol {
  const initial = "0"
  const carBodyChecked = "1"
  const engineChecked = "2"
  const carChecked = "3"
  export const carBodyCheckerRole = "CarBodyChecker"
  export const engineCheckerRole = "EngineChecker"
  export const windowCheckerRole = "WindowChecker"
  export const wheelCheckerRole = "WheelChecker"
  export const qualityControlRole = "QualityControl"
  export const cmdCheckCarBody = "carBodyDone"
  export const cmdCheckEngine = "checkEngine"
  export const cmdWheelsDone = "wheelsDone"
  export const cmdWindowsDone = "windowsDone"
  export const cmdCheckCar = "checkCar"

  export const protocol: SwarmProtocolType = {
    initial: initial,
    transitions: [
      {source: initial, target: carBodyChecked, label: {cmd: cmdCheckCarBody, role: carBodyCheckerRole, logType: [Events.carBody.type]}},
      {source: carBodyChecked, target: engineChecked, label: {cmd: cmdCheckEngine, role: engineCheckerRole, logType: [Events.engineChecked.type]}},
      {source: engineChecked, target: engineChecked, label: {cmd: cmdWheelsDone, role: wheelCheckerRole, logType: [Events.wheelsDone.type]}},
      {source: engineChecked, target: engineChecked, label: {cmd: cmdWindowsDone, role: windowCheckerRole, logType: [Events.windowsDone.type]}},
      {source: engineChecked, target: carChecked, label: {cmd: cmdCheckCar, role: qualityControlRole, logType: [Events.finishedCar.type]}}
    ]
  }

  const subscriptionsResult: DataResult<Subscriptions>
    = overapproxWFSubscriptions([protocol], {}, 'TwoStep')
  if (subscriptionsResult.type === 'ERROR') throw new Error(subscriptionsResult.errors.join(', '))
  export const subscriptions: Subscriptions = subscriptionsResult.data
}


// Machine adaptation did not go well when switching the order of warehouse and engine installer. Why?
// Not minimized?
// throw new Error(`${firstTrigger.type} has been registered as a reaction guard for this state.`);
export const carFactoryProtocol: InterfacingProtocols = [
  SteelPressProtocol.protocol,
  PaintShopProtocol.protocol,
  EngineInstallationProtocol.protocol,
  WarehouseProtocol.protocol,
  WheelInstallationProtocol.protocol,
  WindowInstallationProtocol.protocol,
  QualityControlProtocol.protocol
]
// Well-formed subscription for the composition protocol
const resultSubsCarFactory: DataResult<Subscriptions>
  = overapproxWFSubscriptions(carFactoryProtocol, {}, 'TwoStep')
if (resultSubsCarFactory.type === 'ERROR') throw new Error(resultSubsCarFactory.errors.join(', '))
export var subsCarFactory: Subscriptions = resultSubsCarFactory.data

// check that the subscription generated for the composition is indeed well-formed
const resultCheckWf: Result = checkComposedSwarmProtocol(carFactoryProtocol, subsCarFactory)
if (resultCheckWf.type === 'ERROR') throw new Error(resultCheckWf.errors.join(', \n'))

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
export function getRandomInt(min: number, max: number) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}

export const printState = (machineName: string, stateName: string, statePayload: any) => {
  console.log(chalk.bgBlack.white.bold(`${machineName} - State: ${stateName}. Payload: ${statePayload ? JSON.stringify(statePayload, null, 0) : "{}"}`))
}

/* for (const p of carFactoryProtocol) {
  console.log(JSON.stringify(p, null, 2))
  console.log("$$$$")
} */

const composeProtocolsResult = composeProtocols(carFactoryProtocol)
if (composeProtocolsResult.type === 'ERROR') throw new Error(composeProtocolsResult.errors.join(", \n"))
//console.log(JSON.stringify(composeProtocolsResult.data, null, 2))

export const getArgs = () => {
    const argv = yargs(hideBin(process.argv))
            .option("displayName", {
              alias: "n",
              type: "string",
              description: "Display name of application",
              default: "car-factory"
            })
            .option("appId", {
              alias: "i",
              type: "string",
              description: "ID of application",
              default: "com.example.car-factory"
            })
            .option("appVersion", {
              alias: "av",
              type: "string",
              description: "Version of application",
              default: "1.0.0"
            })
            .parseSync();
    return argv
}

type Argv = {
  displayName: string;
  appId: string;
  appVersion: string;
}
export const manifestFromArgs = (argv: Argv): AppManifest => {
  return { appId: argv.appId, displayName: argv.displayName, version: argv.appVersion}
}