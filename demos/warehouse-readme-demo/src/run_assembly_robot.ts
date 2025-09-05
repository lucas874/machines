import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { manifest, Composition, printState, Events } from './protocol'
import chalk from "chalk";
import { Assemble, assemblyRobotAdapted, initialAssemblyAdapted } from './assembly_robot';

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory')
  const assemblyRobot = createMachineRunnerBT(app, tags, initialAssemblyAdapted, undefined, assemblyRobotAdapted)
  printState(assemblyRobotAdapted.machineName, initialAssemblyAdapted.mechanism.name, undefined)

  for await (const state of assemblyRobot) {
    if (state.isLike(Assemble)) {
      await state.cast().commands()?.assemble()
    }
  }
  app.dispose()
}

main()