import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_protocol, getRandomInt, factory_protocol, subs_factory, print_event, subs_composition, projectionInfoRobot, printState  } from './protocol'
import { checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'
import * as readline from 'readline';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of robot in factory w.r.t. subs_factory is:
const robot = Composition.makeMachine('Robot')
// slide with code and next to it depiction of machine. ecoop 23 does this

export const s0 = robot.designEmpty('s0').finish()

export const s1 = robot.designState('s1').withPayload<{partName: string}>()
  .command("build", [Events.car], (s: any) => {
    var modelName = s.self.partName === 'spoiler' ? "sports car" : "sedan";
    return [Events.car.make({partName: s.self.partName, modelName: modelName})]})
  .finish()

export const s2 = robot.designEmpty('s2').finish()

s0.react([Events.part], s1, (_, e) => {
  return s1.make({partName: e.payload.partName})})

s1.react([Events.car], s2, (_, e) => { return s2.make()})

// Adapt machine
const [factoryRobotAdapted, s0Adapted] = Composition.adaptMachine('Robot', projectionInfoRobot, Events.allEvents, s0, true)

// Run the adapted machine
async function main() {

  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, projectionInfoRobot.branches, projectionInfoRobot.specialEventTypes)
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
  rl.close();
  app.dispose()
}

main()