import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_quality_protocol, getRandomInt, factory_protocol, subs_factory, print_event, subs_composition, printState  } from './protocol'
import { checkComposedProjection, projectionAndInformation, projectionAndInformationNew } from '@actyx/machine-check'
import * as readline from 'readline';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of robot in factory w.r.t. subs_factory is:
const robot = Composition.makeMachine('R')
export const s0 = robot.designEmpty('s0').finish()
export const s1 = robot.designState('s1').withPayload<{part: string}>()
  .command("build", [Events.car], (s: any) => {
    var modelName = s.self.part === 'spoiler' ? "sports car" : "sedan";
    return [Events.car.make({part: s.self.part, modelName: modelName})]})
  .finish()
export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.partOK], s1, (_, e) => {
  return s1.make({part: e.payload.part})})
s1.react([Events.car], s2, (_) => s2.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(factory_protocol, subs_factory, "R", robot.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Projection of warehouse || factory || quality over R
const projectionInfoResult = projectionAndInformation(warehouse_factory_quality_protocol, subs_composition, "R")
//const projectionInfoResult = projectionAndInformationNew(warehouse_factory_quality_protocol, subs_composition, "R", robot.createJSONForAnalysis(s0), 1)
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data
//console.log(JSON.stringify(projectionInfo1, null, 2))

// Adapt machine
const [factoryRobotAdapted, s0Adapted] = Composition.adaptMachine("Robot", projectionInfo, Events.allEvents, s0, true)

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)
    printState(factoryRobotAdapted.machineName, s0Adapted.mechanism.name, undefined)

    for await (const state of machine) {
      if(state.isLike(s1)) {
        rl.on('line', (_) => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s1)) {
            stateAfterTimeOut?.cast().commands()?.build()
          }
        })
      }
    }
    app.dispose()
}

main()