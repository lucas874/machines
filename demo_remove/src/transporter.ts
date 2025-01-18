import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections, getRandomInt  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'

const transporter = Composition.makeMachine('T')
export const s0 = transporter.designEmpty('s0')
    .command('request', [Events.partID], () => {var id = "tire"; console.log("requesting: ", id); return [Events.partID.make({id: id})]})
    .finish()
export const s1 = transporter.designEmpty('s1').finish()
export const s2 = transporter.designEmpty('s2')
    .command('deliver', [Events.part], (s,e) => {console.log("s is: ", s); console.log("e is : ", e); return [Events.part.make({part: "dsasda"})]})
    .finish()
export const s3 = transporter.designEmpty('s3').finish()

s0.react([Events.partID], s1, (_) => s1.make())
s0.react([Events.time], s3, (_) => s3.make())
s1.react([Events.position], s2, (_) => s2.make())
s2.react([Events.part], s0, (_) => s0.make())

const result_projection = projectCombineMachines(interfacing_swarms, subs, "T")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
cMap.set(Events.partID.type, (s: any, e: any) => {console.log(s, e); var id = s.self.part; console.log("requesting: ", id); return [Events.partID.make({id: id})]})
cMap.set(Events.part.type, (s: any, e: any) => {return [Events.part.make({part: s.self.part})]})

const rMap = new Map()
const positionReaction : ProjMachine.ReactionEntry = {
  identifiedByInput: true,
  genPayloadFun: (_, e) => { console.log("got a ", e.payload.part); return {part: e.payload.part} }
}
rMap.set(Events.position.type, positionReaction)
const fMap : any = {commands: cMap, reactions: rMap}

const mAnalysisResource: MachineAnalysisResource = {initial: projection.initial, subscriptions: [], transitions: projection.transitions}
const [m3, i3] = Composition.extendMachine("T", mAnalysisResource, Events.allEvents, fMap)

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const parts = ['tire', 'windshield', 'chassis', 'hood']
    const machine = createMachineRunner(app, tags, i3, {part: parts[Math.floor(Math.random() * parts.length)]})

    for await (const state of machine) {
      console.log("state is: ", state)

      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'request') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1).includes('request')) {
                    s1.request()
                }
            }, getRandomInt(2000, 5000))
            break
          }
          if (c === 'deliver') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1).includes('deliver')) {
                    s1.deliver()
                }
            }, getRandomInt(2000, 5000))
            break
          }
      }
    }
    app.dispose()
}

main()