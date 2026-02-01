import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { manifest, printState, machineRunnerProtoName } from './protocol'
import { Assemble, AssemblyLine, assemblyRobotAdapted, initialAssemblyAdapted } from './assembly_robot';

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = AssemblyLine.tagWithEntityId(machineRunnerProtoName)
  const assemblyRobot = createMachineRunnerBT(app, tags, initialAssemblyAdapted, undefined, assemblyRobotAdapted)
  printState(assemblyRobotAdapted.machineName, initialAssemblyAdapted.mechanism.name, undefined)

  for await (const state of assemblyRobot) {
    if (state.isLike(Assemble)) {
      setTimeout(() => {
        const stateAfterTimeOut = assemblyRobot.get()
        if (stateAfterTimeOut?.isLike(Assemble)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.assemble()
        }
      }, 1000) 
    }
    if (state.isFinal()) {
      console.log("Final state reached, press CTRL + C to quit.")
    }
  }
  app.dispose()
}

main()