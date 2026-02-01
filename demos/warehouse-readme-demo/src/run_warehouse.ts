import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { manifest, TransportOrder, printState, Events, machineRunnerProtoName } from './protocol'
import chalk from "chalk";
import { AcknowledgeWarehouse, DoneWarehouse, InitialWarehouse, warehouseAdapted, initialWarehouseAdapted } from './warehouse';

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = TransportOrder.tagWithEntityId(machineRunnerProtoName)
  const warehouse = createMachineRunnerBT(app, tags, initialWarehouseAdapted, undefined, warehouseAdapted)
  printState(warehouseAdapted.machineName, initialWarehouseAdapted.mechanism.name, undefined)
  console.log()
  console.log(chalk.bgBlack.red.dim`    ${Events.request.type}!`);

  for await (const state of warehouse) {
    if (state.isLike(InitialWarehouse)) {
      setTimeout(() => {
        const stateAfterTimeOut = warehouse.get()
        if (stateAfterTimeOut?.isLike(InitialWarehouse)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.request(parts[Math.floor(Math.random() * parts.length)], "a", "b")
        }
      }, 1000)
    }
    if (state.isLike(AcknowledgeWarehouse)) {
      setTimeout(() => {
        const stateAfterTimeOut = warehouse.get()
        if (stateAfterTimeOut?.isLike(AcknowledgeWarehouse)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.acknowledge()
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