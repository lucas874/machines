import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { manifest, TransportOrder, printState, Events } from './protocol'
import chalk from "chalk";
import { AcknowledgeWarehouse, DoneWarehouse, InitialWarehouse, warehouseAdapted, warehouseInitialAdapted } from './warehouse';

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = TransportOrder.tagWithEntityId('warehouse-factory')
  const warehouse = createMachineRunnerBT(app, tags, warehouseInitialAdapted, undefined, warehouseAdapted)
  printState(warehouseAdapted.machineName, warehouseInitialAdapted.mechanism.name, undefined)
  console.log()
  console.log(chalk.bgBlack.red.dim`    ${Events.request.type}!`);

  for await (const state of warehouse) {
    if (state.isLike(InitialWarehouse)) {
      await state.cast().commands()?.request(parts[Math.floor(Math.random() * parts.length)], "a", "b")
    }
    if (state.isLike(AcknowledgeWarehouse)) {
      await state.cast().commands()?.acknowledge()
    }
  }
  app.dispose()
}

main()