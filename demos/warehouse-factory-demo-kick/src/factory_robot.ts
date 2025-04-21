import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms,getRandomInt, print_event  } from './protocol'
import { checkWWFSwarmProtocol, checkComposedProjection, Subscriptions, ResultData, overapproxWWFSubscriptions, projectionAndInformation } from '@actyx/machine-check'

const robotFinal = "{ { { 3 } } || { { 0 } }, { { 3 } } || { { 3 } } }"

// Generate a subscription w.r.t. which Gwarehouse || Gfactory || Gquality is well-formed
const result_sub: ResultData<Subscriptions>
  = overapproxWWFSubscriptions(interfacing_swarms, {}, 'Medium')
if (result_sub.type === 'ERROR') throw new Error(result_sub.errors.join(', '))
export const sub: Subscriptions = result_sub.data

// Check well-formedness (should be WF since we generated subscription using overapproxWWFSubscriptions(...), only here for demonstration purposes)
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

s0.react([Events.partOK], s1, (_, e) => {
  print_event(e);
  console.log("received a ", e.payload.part);
  return s1.make({part: e.payload.part})})
s1.react([Events.car], s2, (_, e) => { print_event(e); return s2.make() })

// Projection of Gwarehouse || Gfactory || Gquality over R
const projectionInfoResult = projectionAndInformation(interfacing_swarms, sub, "R")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Extend machine
const [factoryRobotAdapted, s0_] = Composition.adaptMachine("R", projectionInfo, Events.allEvents, s0)

// Check machine (for demonstration purposes)
const checkProjResult = checkComposedProjection(interfacing_swarms, sub, "R", factoryRobotAdapted.createJSONForAnalysis(s0_))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

    for await (const state of machine) {
      console.log("Robot. State is:", state.type)
      if (state.payload !== undefined) {
        console.log("State payload is:", state.payload)
      }
      console.log()
      if (state.type === robotFinal) {
        console.log("\x1b[32mRobot reached its final state. Press CTRL + C to exit.\x1b[0m")
      }
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