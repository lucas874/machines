import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_protocol, getRandomInt, factory_protocol, subs_factory, print_event, subs_composition  } from './protocol'
import { checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'
import * as readline from 'readline';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of robot in factory w.r.t. subs_factory is:
const robot = Composition.makeMachine('R')
export const s0 = robot.designEmpty('s0').finish()
export const s1 = robot.designState('s1').withPayload<{partName: string}>()
  .command("build", [Events.car], (s: any) => {
    var modelName = s.self.partName === 'spoiler' ? "sports car" : "sedan";
    console.log("using the ", s.self.partName, " to build a ", modelName);
    return [Events.car.make({partName: s.self.part, modelName: modelName})]})
  .finish()
export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.part], s1, (_, e) => {
  print_event(e);
  console.log("received a ", e.payload.partName);
  return s1.make({partName: e.payload.partName})})
s1.react([Events.car], s2, (_, e) => {print_event(e); return s2.make()})

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(factory_protocol, subs_factory, "R", robot.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Projection of warehouse || factory over R
const projectionInfoResult = projectionAndInformation(warehouse_factory_protocol, subs_composition, "R")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapt machine
const [factoryRobotAdapted, s0_] = Composition.adaptMachine("R", projectionInfo, Events.allEvents, s0)

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory-quality')
  const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

  for await (const state of machine) {
    console.log("Robot. State is:", state.type)
    if (state.payload !== undefined) {
      console.log("State payload is:", state.payload)
    }
    console.log()
    if(state.isLike(s1)) {
      rl.question("Invoke build? ", (_) => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          stateAfterTimeOut?.cast().commands()?.build()
        }
      })
    }
  }
  rl.close();
  app.dispose()
}

main()