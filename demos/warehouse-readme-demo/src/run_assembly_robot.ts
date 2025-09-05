import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { manifest, printState, Events } from './protocol'
import chalk from "chalk";
import { Assemble, AssemblyProtocol, assemblyRobotAdapted, initialAssemblyAdapted } from './assembly_robot';

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = AssemblyProtocol.tagWithEntityId('warehouse-factory')
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