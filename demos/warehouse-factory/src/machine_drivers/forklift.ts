import { Actyx, Tags } from '@actyx/sdk'
import { createMachineRunnerBT, MachineRunner } from '@actyx/machine-runner'
import { manifest, Protocol, printState, machineRunnerProtoName, logToFile } from '../protocol'
import { isValidVersion, VersionSelector } from '../version_selector'
import { forkliftWarehouseFactory, initialStateWarehouseFactory, deliverState, forkliftBT, initialStateBT, initialStateWarehouseFactoryQuality, forkliftWarehouseFactoryQuality } from '../machines/forklift_machine'

// Run a forklift machine
async function main() {
  if (!isValidVersion(process.argv[2])) {
    throw Error(`Invalid version: ${process.argv[2]}`)
  }
  const version = process.argv[2]
  const app = await Actyx.of(manifest)
  const tags = Protocol.tagWithEntityId(machineRunnerProtoName)
  const machine = selectMachine(version, app, tags)

  for await (const state of machine) {
    if (state.isLike(deliverState)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(deliverState)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.get()
        }
      }, 3500)
    }
    if (state.isFinal()) {
      if (version === VersionSelector.KickTheTires) {
        logToFile(process.argv[3], "Forklift is ok.")
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
      printState(forkliftBT.machineName, initialStateBT.mechanism.name, undefined)
      return createMachineRunnerBT(app, tags, initialStateBT, undefined, forkliftBT)
    case VersionSelector.KickTheTires:
    case VersionSelector.WarehouseFactory:
      printState(forkliftWarehouseFactory.machineName, initialStateWarehouseFactory.mechanism.name, undefined)
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactory, undefined, forkliftWarehouseFactory)
    case VersionSelector.WarehouseFactoryQuality:
        printState(forkliftWarehouseFactoryQuality.machineName, initialStateWarehouseFactoryQuality.mechanism.name, undefined)
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactoryQuality, undefined, forkliftWarehouseFactoryQuality)
    default:
      throw Error(`Invalid version: ${version}`)
  }
}

main()