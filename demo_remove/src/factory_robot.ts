import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections, getRandomInt  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'

const robot = Composition.makeMachine('R')
export const s0 = robot.designEmpty('s0').finish()
export const s1 = robot.designEmpty('s1')
    .command("build", [Events.car], (s,e) => {return [Events.car.make({part: "dsasda", modelName: "sda"})]})
    .finish()
export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.part], s1, (_) => s1.make())
s1.react([Events.car], s2, (_) => s2.make())

const result_projection = projectCombineMachines(interfacing_swarms, subs, "R")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
cMap.set(Events.car.type, (s: any, _: any) => {var modelName = "sedan"; console.log("got a: ", s.self.part, " using it to build a ", modelName); return [Events.car.make({part: s.self.part, modelName: modelName})]})
const rMap = new Map()
const partReaction : ProjMachine.ReactionEntry = {
  identifiedByInput: true,
  genPayloadFun: (_, e) => { console.log("got a ", e.payload.part); return {part: e.payload.part} }
}
rMap.set(Events.part.type, partReaction)
const fMap : any = {commands: cMap, reactions: rMap}
const mAnalysisResource: MachineAnalysisResource = {initial: projection.initial, subscriptions: [], transitions: projection.transitions}
const [m3, i3] = Composition.extendMachine("R", mAnalysisResource, Events.allEvents, fMap)
//console.log(m3.createJSONForAnalysis(i3))
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunner(app, tags, i3, undefined)
    //var hasRequested = false
    //var isDone = false
    for await (const state of machine) {
      console.log("state is: ", state)
      /* if (isDone) {
          console.log("shutting down")
          break
      } */

      const s = state.cast()
      for (var c in s.commands()) {
          var cmds = s.commands() as any;
          if (c === 'build') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1).includes('build')) { //console.log(Object.keys(s1))
                    s1.build()
                }
            }, getRandomInt(2000, 5000))
            break
          }
      }
    }
    app.dispose()
}

main()