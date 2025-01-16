import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'
/* type partIDToGet = {
    pid: string
} */
const transporter = Composition.makeMachine('T')
export const s0 = transporter.designEmpty('s0')
    .command('request', [Events.partID], () => [{}])
    .finish()
export const s1 = transporter.designEmpty('s1').finish()
export const s2 = transporter.designEmpty('s2')
    .command('deliver', [Events.part], () => [{}])
    .finish()
export const s3 = transporter.designEmpty('s3').finish()
/* console.log("sub comp: ", JSON.stringify(subs))
console.log("sub wh: ", JSON.stringify(subswh))
console.log("sub f: ", JSON.stringify(subsf)) */



s0.react([Events.partID], s1, (_) => s1.make())
s0.react([Events.time], s3, (_) => s3.make())
s1.react([Events.position], s2, (_) => s2.make())
s2.react([Events.part], s0, (_) => s0.make())

const result_projection = projectCombineMachines(interfacing_swarms, subs, "T")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
const rMap = new Map()
const statePayloadMap = new Map()
const fMap : any = {commands: cMap, reactions: rMap, statePayloads: statePayloadMap}
const mAnalysisResource: MachineAnalysisResource = {initial: projection.initial, subscriptions: [], transitions: projection.transitions}
const [m3, i3] = Composition.extendMachine("D", mAnalysisResource, Events.allEvents, [transporter, s0], fMap)
console.log(m3.createJSONForAnalysis(i3))