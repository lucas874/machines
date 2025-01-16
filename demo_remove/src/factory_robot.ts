import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'

const robot = Composition.makeMachine('R')
export const s0 = robot.designEmpty('s0').finish()
export const s1 = robot.designEmpty('s1')
    .command("build", [Events.car], () => [{}])
    .finish()
export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.part], s1, (_) => s1.make())
s1.react([Events.car], s2, (_) => s2.make())

const result_projection = projectCombineMachines(interfacing_swarms, subs, "R")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
const rMap = new Map()
const statePayloadMap = new Map()
const fMap : any = {commands: cMap, reactions: rMap, statePayloads: statePayloadMap}
const mAnalysisResource: MachineAnalysisResource = {initial: projection.initial, subscriptions: [], transitions: projection.transitions}
const [m3, i3] = Composition.extendMachine("R", mAnalysisResource, Events.allEvents, [robot, s0], fMap)
console.log(m3.createJSONForAnalysis(i3))