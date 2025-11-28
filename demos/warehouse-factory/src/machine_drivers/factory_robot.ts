import { Actyx, Tags } from '@actyx/sdk'
import { createMachineRunnerBT, MachineRunner } from '@actyx/machine-runner'
import { manifest, Protocol, printState, getRandomInt, machineRunnerProtoName, logToFile } from '../protocol'
import { initialStateWarehouseFactory, robotWarehouseFactory, buildState, initialStateWarehouseFactoryQuality, robotWarehouseFactoryQuality } from '../machines/robot_machine';
import { isValidVersion, VersionSelector } from '../version_selector';

// Run the adapted machine
async function main() {
  if (!isValidVersion(process.argv[2])) {
    throw Error(`Invalid version: ${process.argv[2]}`)
  }
  const version = process.argv[2]
  const app = await Actyx.of(manifest)
  const tags = Protocol.tagWithEntityId(machineRunnerProtoName)
  const machine = selectMachine(version, app, tags)

  for await (const state of machine) {
    if(state.isLike(buildState)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(buildState)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.buildCar()
        }
      }, getRandomInt(3000, 4500))
    }
    if (state.isFinal()) {
      if (version === VersionSelector.KickTheTires) {
        logToFile(process.argv[3], "Factory robot is ok.")
      }
      console.log("Final state reached, press CTRL + C to quit.")
    }
  }
  app.dispose()
}

// We use the same underlying machine, but instantiate it differently (e.g., different compositions)
const selectMachine = <
  MachineName extends string,
  State
>(
  version: VersionSelector,
  app: Actyx,
  tags: Tags<any>
): MachineRunner<typeof machineRunnerProtoName, MachineName, State> => {
  switch (version) {
    case VersionSelector.KickTheTires:
    case VersionSelector.WarehouseFactory:
      printState(robotWarehouseFactory.machineName, initialStateWarehouseFactory.mechanism.name, undefined)
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactory, undefined, robotWarehouseFactory)
    case VersionSelector.WarehouseFactoryQuality:
      printState(robotWarehouseFactoryQuality.machineName, initialStateWarehouseFactoryQuality.mechanism.name, undefined)
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactoryQuality, undefined, robotWarehouseFactoryQuality)
    default:
      throw Error(`Invalid version: ${version}`)
  }
}

main()