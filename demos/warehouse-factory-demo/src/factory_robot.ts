import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_protocol,getRandomInt, factory_protocol, subs_factory  } from './protocol'
import { checkWWFSwarmProtocol, checkComposedProjection, Subscriptions, ResultData, overapproxWWFSubscriptions, projectionAndInformation } from '@actyx/machine-check'

// Generate a subscription w.r.t. which Gwarehouse || Gfactory || Gquality is well-formed
const result_sub: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(warehouse_factory_protocol, {}, 'Medium')
if (result_sub.type === 'ERROR') throw new Error(result_sub.errors.join(', '))
export const sub: Subscriptions = result_sub.data

// Check well-formedness (should be WF since we generated subscription using overapproxWWFSubscriptions(...), only here for demonstration purposes)
const checkResult = checkWWFSwarmProtocol(warehouse_factory_protocol, sub)
if (checkResult.type == 'ERROR') throw new Error(checkResult.errors.join(", "))

// Using the machine runner DSL an implmentation of robot in factory w.r.t. subs_factory is:
const robot = Composition.makeMachine('R')
export const s0 = robot.designEmpty('s0').finish()
export const s1 = robot.designState('s1').withPayload<{part: string}>()
  .command("build", [Events.car], (s: any, _: any) => {
    var modelName = s.self.part === 'spoiler' ? "sports car" : "sedan";
    console.log("using the ", s.self.part, " to build a ", modelName);
    return [Events.car.make({part: s.self.part, modelName: modelName})]})
  .finish()
export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.partOK], s1, (_, e) => {
  console.log("received a ", e.payload.part);
  return s1.make({part: e.payload.part})})
s1.react([Events.car], s2, (_) => s2.make())

// Check that the original machine is a correct implementation. A prerequiste for reusing it.
const checkProjResult = checkComposedProjection(factory_protocol, subs_factory, "R", robot.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Projection of warehouse || factory over R
const projectionInfoResult = projectionAndInformation(warehouse_factory_protocol, sub, "R")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapt machine
const [factoryRobotAdapted, s0_] = Composition.adaptMachine("R", projectionInfo, Events.allEvents, s0)

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

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