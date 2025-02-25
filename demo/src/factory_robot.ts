import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms,getRandomInt  } from './factory_protocol'
import { projectCombineMachines, checkWWFSwarmProtocol, checkComposedProjection, Subscriptions, ResultData, InterfacingSwarms, overapproxWWFSubscriptions } from '@actyx/machine-check'


// Generate a subscription w.r.t. which Gwarehouse || Gfactory || Gquality is well-formed
const result_sub: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarms, {}, 'Medium')
if (result_sub.type === 'ERROR') throw new Error(result_sub.errors.join(', '))
export const sub: Subscriptions = result_sub.data


// Check well-formedness (only here for demonstration purposes)
const checkResult = checkWWFSwarmProtocol(interfacing_swarms, sub)
if (checkResult.type == 'ERROR') throw new Error(checkResult.errors.join(", "))























// Using the machine runner DSL an implmentation of robot in Gfactory is:
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















// Projection of Gwarehouse || Gfactory || Gquality over R
const result_projection = projectCombineMachines(interfacing_swarms, sub, "R")
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
const fMap = {commands: cMap, reactions: rMap, initialPayloadType: undefined}


















// Extend machine
const [m3, i3] = Composition.extendMachine("R", projection, Events.allEvents, fMap)
























// Check machine (for demonstration purposes)
const checkProjResult = checkComposedProjection(interfacing_swarms, sub, "R", m3.createJSONForAnalysis(i3))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))






















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
                if (Object.keys(s1 || {}).includes('build')) {
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