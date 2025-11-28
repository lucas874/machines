import { Actyx, Tags } from '@actyx/sdk'
import { createMachineRunnerBT, MachineRunner} from '@actyx/machine-runner'
import { Events, manifest, Protocol, printState, getRandomInt, machineRunnerProtoName, logToFile } from '../protocol'
import { initialStateBT, doorBT, initialState, initialStateWarehouseFactory, doorWarehouseFactory, initialStateWarehouseFactoryQuality, doorWarehouseFactoryQuality } from '../machines/door_machine';
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
    if (state.isLike(initialState)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(initialState)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.closeDoor()
        }
      }, getRandomInt(4500, 5500))
    }
    if (state.isFinal()) {
      if (version === VersionSelector.KickTheTires) {
        logToFile(process.argv[3], "Door is ok.")
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
    case VersionSelector.Warehouse:
      printState(doorBT.machineName, initialStateBT.mechanism.name, undefined, [Events.closingTimeEvent.type])
      return createMachineRunnerBT(app, tags, initialStateBT, undefined, doorBT)
    case VersionSelector.KickTheTires:
    case VersionSelector.WarehouseFactory:
      printState(doorWarehouseFactory.machineName, initialStateWarehouseFactory.mechanism.name, undefined, [Events.closingTimeEvent.type])
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactory, undefined, doorWarehouseFactory)
    case VersionSelector.WarehouseFactoryQuality:
      printState(doorWarehouseFactoryQuality.machineName, initialStateWarehouseFactoryQuality.mechanism.name, undefined, [Events.closingTimeEvent.type])
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactoryQuality, undefined, doorWarehouseFactoryQuality)
    default:
      throw Error(`Invalid version: ${version}`)
  }
}

main()