import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_quality_protocol, getRandomInt, factory_protocol, subs_factory, print_event, subs_composition  } from './protocol'
import { checkComposedProjection } from '@actyx/machine-check'

// Using the machine runner DSL an implmentation of robot in factory w.r.t. subs_factory is:
const robot = Composition.makeMachine('R')
export const s0 = robot.designEmpty('s0').finish()
export const s1 = robot.designState('s1').withPayload<{part: string}>()
  .command("build", [Events.car], (s: any) => {
    var modelName = s.self.part === 'spoiler' ? "sports car" : "sedan";
    console.log("using the ", s.self.part, " to build a ", modelName);
    return [Events.car.make({part: s.self.part, modelName: modelName})]})
  .finish()
export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.partOK], s1, (_, e) => {
  print_event(e);
  console.log("received a ", e.payload.part);
  return s1.make({part: e.payload.part})})
s1.react([Events.car], s2, (_) => s2.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(factory_protocol, subs_factory, "R", robot.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapt machine
const [factoryRobotAdapted, s0Adapted] = Composition.adaptMachine('R', warehouse_factory_quality_protocol, 1, subs_composition, [robot, s0]).data!

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, factoryRobotAdapted)

    for await (const state of machine) {
      console.log("Robot. State is:", state.type)
      if (state.payload !== undefined) {
        console.log("State payload is:", state.payload)
      }
      console.log()
      if(state.isLike(s1)) {
        setTimeout(() => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s1)) {
            stateAfterTimeOut?.cast().commands()?.build()
          }
        }, getRandomInt(4000, 8000))
      }
    }
    app.dispose()
}

main()