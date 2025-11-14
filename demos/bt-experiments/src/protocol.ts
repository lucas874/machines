/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'
import { SwarmProtocolType, Subscriptions, Result, DataResult, overapproxWFSubscriptions, checkComposedSwarmProtocol, InterfacingProtocols, exactWFSubscriptions } from '@actyx/machine-check'
import chalk from "chalk";

export const manifest = {
  appId: 'com.example.bt-experiment',
  displayName: 'BT Experiment',
  version: '1.0.0',
}

type ClosingTimePayload = { timeOfDay: string }
type PartIDPayload = {partName: string}
type PosPayload = {position: string, partName: string}
type PartPayload = {partName: string}
type CarPayload = {partName: string, modelName: string}

export namespace Events {
  export const A = MachineEvent.design('A').withoutPayload()
  export const B = MachineEvent.design('B').withoutPayload()
  export const C = MachineEvent.design('C').withoutPayload()
  export const D = MachineEvent.design('D').withoutPayload()
  export const E = MachineEvent.design('E').withoutPayload()
  export const I1 = MachineEvent.design('I1').withoutPayload()
  export const I2 = MachineEvent.design('I2').withoutPayload()

  export const allEvents = [A, B, C, D, E, I1, I2] as const
}

export const Composition = SwarmProtocol.make('Composition', Events.allEvents)

export const proto1 =   {
  initial: "0",
  transitions: [
    {
      label: {
        cmd: "cmdA",
        logType: [
          Events.A.type
        ],
        role: "roleA"
      },
      source: "0",
      target: "1"
    },
    {
      label: {
        cmd: "cmdB",
        logType: [
          Events.B.type
        ],
        role: "roleA"
      },
      source: "0",
      target: "5"
    },
    {
      label: {
        cmd: "cmdI1",
        logType: [
          Events.I1.type
        ],
        role: "roleInterface"
      },
      source: "1",
      target: "2"
    },
    {
      label: {
        cmd: "cmdI2",
        logType: [
          Events.I2.type
        ],
        role: "roleInterface"
      },
      source: "2",
      target: "3"
    },
    {
      label: {
        cmd: "cmdC",
        logType: [
          "C"
        ],
        role: "roleA"
      },
      source: "3",
      target: "4"
    }
  ]
}


export const proto2 = {
  initial: "0",
  transitions: [
    {
      label: {
        cmd: "cmdI1",
        logType: [
          Events.I1.type
        ],
        role: "roleInterface"
      },
      source: "0",
      target: "1"
    },
    {
      label: {
        cmd: "cmdE",
        logType: [
          Events.E.type
        ],
        role: "roleD"
      },
      source: "1",
      target: "4"
    },
    {
      label: {
        cmd: "cmdD",
        logType: [
          Events.D.type
        ],
        role: "roleD"
      },
      source: "1",
      target: "2"
    },
    {
      label: {
        cmd: "cmdI2",
        logType: [
          Events.I2.type
        ],
        role: "roleInterface"
      },
      source: "2",
      target: "3"
    }
  ]
}

export const protocol_1: InterfacingProtocols = [proto1]
export const protocol_2: InterfacingProtocols = [proto2]
export const interfacing_protocols: InterfacingProtocols = [proto1, proto2]

// Well-formed subscription for the warehouse protocol
const result_subs_proto1: DataResult<Subscriptions>
  = exactWFSubscriptions(protocol_1, {})
if (result_subs_proto1.type === 'ERROR') throw new Error(result_subs_proto1.errors.join(', '))
export var subs_proto1: Subscriptions = result_subs_proto1.data

// Well-formed subscription for the factory protocol
const result_subs_proto2: DataResult<Subscriptions>
  = exactWFSubscriptions(protocol_2, {})
if (result_subs_proto2.type === 'ERROR') throw new Error(result_subs_proto2.errors.join(', '))
export var subs_proto2: Subscriptions = result_subs_proto2.data

// Well-formed subscription for the warehouse || factory protocol
const result_subs_composition: DataResult<Subscriptions>
  = overapproxWFSubscriptions(interfacing_protocols, {}, 'TwoStep')
if (result_subs_composition.type === 'ERROR') throw new Error(result_subs_composition.errors.join(', '))
export var subs_composition: Subscriptions = result_subs_composition.data

// outcomment the line below to make well-formedness check fail
//subs_composition['FL'] = ['pos']

// check that the subscription generated for the composition is indeed well-formed
const result_check_wf: Result = checkComposedSwarmProtocol(interfacing_protocols, subs_composition)
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