import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'

/*

Using the machine runner DSL an implmentation of robot in Gfactory is:

const robot = Composition.makeMachine('R')
export const s0 = robot.designEmpty('s0').finish()
export const s1 = robot.designState('s1').withPayload<{part: string}>()
  .command("build", [Events.car], (s: any, _: any) => {
    var modelName = s.self.part === 'spoiler' ? "sports car" : "sedan";
    console.log("using the ", s.self.part, " to build a ", modelName);
    return [Events.car.make({part: s.self.part, modelName: modelName})]})
  .finish()
export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.part], s1, (_, e) => {
  console.log("received a ", e.payload.part);
  return s1.make({part: e.payload.part})})
s1.react([Events.car], s2, (_) => s2.make())

*/

// With our extension of the library we create a map from events to reactions
// and commands instead and use the projection of the composition over
// the role to create the extended machine

// Projection of Gwarehouse || Gfactory || Gquality over R
const result_projection = projectCombineMachines(interfacing_swarms, subs, "R")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

// Command map
const cMap = new Map()
cMap.set(Events.car.type,
  (s: any, _: any) => {
    var modelName = s.self.part === "spoiler" ? "sports car" : "sedan";
    console.log("using the ", s.self.part, " to build a ", modelName);
    return [Events.car.make({part: s.self.part, modelName: modelName})]})

// Reaction map
const rMap = new Map()
const partReaction : ProjMachine.ReactionEntry = {
  genPayloadFun: (_, e) => {
    console.log("received a ", e.payload.part);
    return {part: e.payload.part} }
}

rMap.set(Events.part.type, partReaction)
const fMap : any = {commands: cMap, reactions: rMap, initialPayloadType: undefined}

// Extended machine
const [m3, i3] = Composition.extendMachine("R", projection, Events.allEvents, fMap)

// Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunner(app, tags, i3, undefined)

    for await (const state of machine) {
      console.log("robot. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'build') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1).includes('build')) {
                    s1.build()
                }
            }, getRandomInt(4000, 8000))
            break
          }
      }
    }
    app.dispose()
}

main()