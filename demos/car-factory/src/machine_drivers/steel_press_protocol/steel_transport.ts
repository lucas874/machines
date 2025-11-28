import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, getRandomInt, SteelPressProtocol, NUMBER_OF_CAR_PARTS, getArgs, manifestFromArgs } from '../../protocol.js'
import chalk from 'chalk';
import { s0, steelTransport } from '../../machines/steel_press_protocol/steel_transport.js';

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [steelTransportAdapted, s0Adapted] = Composition.adaptMachine(SteelPressProtocol.steelTransportRole, carFactoryProtocol, 0, subsCarFactory, [steelTransport, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const initialPayload = { steelRollsDelivered: 0 }
  const machine = createMachineRunnerBT(app, tags, s0Adapted, initialPayload, steelTransportAdapted)
  printState(steelTransportAdapted.machineName, s0Adapted.mechanism.name, initialPayload)
  console.log(chalk.bgBlack.red.dim`    SteelRoll!`);

  for await (const state of machine) {
    if (state.isLike(s0)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s0) && state.cast().payload.steelRollsDelivered < NUMBER_OF_CAR_PARTS) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.pickUpSteelRoll()
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