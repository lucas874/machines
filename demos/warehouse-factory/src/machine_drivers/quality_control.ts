import { Actyx, Tags } from "@actyx/sdk"
import { Events, getRandomInt, machineRunnerProtoName, manifest, printState, Protocol } from "../protocol"
import { createMachineRunnerBT, MachineRunner } from "@actyx/machine-runner"
import { initialState, initialStateWarehouseFactoryQuality, qualityWarehouseFactoryQuality, reportingState } from "../machines/quality_control_machine"
import { isValidVersion, VersionSelector } from '../version_selector'

// Run the extended machine
async function main() {
  if (!isValidVersion(process.argv[2])) {
    throw Error(`Invalid version: ${process.argv[2]}`)
  }
  const app = await Actyx.of(manifest)
  const tags = Protocol.tagWithEntityId('warehouse-factory')
  const machine = selectMachine(process.argv[2], app, tags)
  printState(qualityWarehouseFactoryQuality.machineName, initialStateWarehouseFactoryQuality.mechanism.name, undefined, [Events.observingEvent.type])
  for await (const state of machine) {
    if(state.isLike(initialState)) {
        setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(initialState)) {
            console.log()
            stateAfterTimeOut?.cast().commands()?.observe()
        }
      }, getRandomInt(3500, 5000))
    }
    if(state.isLike(reportingState)) {
        setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(reportingState)) {
            console.log()
            stateAfterTimeOut?.cast().commands()?.testCar()
        }
      }, getRandomInt(3500, 5000))
    }
    if (state.isFinal()) {
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
    case VersionSelector.WarehouseFactoryQuality:
      return createMachineRunnerBT(app, tags, initialStateWarehouseFactoryQuality, undefined, qualityWarehouseFactoryQuality)
    default:
      throw Error(`Invalid version: ${version}`)
  }
}

main()