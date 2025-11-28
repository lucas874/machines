import { Actyx, Tags } from '@actyx/sdk'
import { createMachineRunnerBT, MachineRunner } from '@actyx/machine-runner'
import { manifest, Protocol, printState, Events, machineRunnerProtoName, logToFile } from '../protocol'
import { initialStateBT, transportBT, initialState, initialStateWarehouseFactory, deliverState, transportWarehouseFactory, initialStateWarehouseFactoryQuality, transportWarehouseFactoryQuality } from '../machines/transport_machine';
import { isValidVersion, VersionSelector } from '../version_selector';

// Run a transport machine
async function main() {
  if (!isValidVersion(process.argv[2])) {
    throw Error(`Invalid version: ${process.argv[2]}`)
  }
  const version = process.argv[2]
  const app = await Actyx.of(manifest)
  const tags = Protocol.tagWithEntityId(machineRunnerProtoName)
  const machine = selectMachine(version, app, tags)

  for await (const state of machine) {
    if(state.isLike(initialState)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(initialState)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.request()
        }
      }, 3500)
    }

    if(state.isLike(deliverState)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(deliverState)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.deliver()
        }
      }, 3500)
    }
    if (state.isFinal()) {
      if (version === VersionSelector.KickTheTires) {
        logToFile(process.argv[3], "Transport is ok.")
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
      printState(transportBT.machineName, initialStateBT.mechanism.name, undefined, [Events.partReqEvent.type])
      return createMachineRunnerBT(app, tags, initialStateBT, undefined, transportBT)
    case VersionSelector.KickTheTires:
    case VersionSelector.WarehouseFactory:
      printState(transportWarehouseFactory.machineName, initialStateWarehouseFactory.mechanism.name, undefined, [Events.partReqEvent.type])
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactory, undefined, transportWarehouseFactory)
    case VersionSelector.WarehouseFactoryQuality:
      printState(transportWarehouseFactoryQuality.machineName, initialStateWarehouseFactoryQuality.mechanism.name, undefined, [Events.partReqEvent.type])
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactoryQuality, undefined, transportWarehouseFactoryQuality)
    default:
      throw Error(`Invalid version: ${version}`)
  }
}

main()