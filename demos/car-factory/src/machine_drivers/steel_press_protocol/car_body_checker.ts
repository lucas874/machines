import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, SteelPressProtocol, NUMBER_OF_CAR_PARTS, getArgs, manifestFromArgs } from '../../protocol.js'
import chalk from "chalk";
import { carBodyChecker, s0 } from '../../machines/steel_press_protocol/car_body_checker.js';

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [carBodyCheckerAdapted, s0Adapted] = Composition.adaptMachine(SteelPressProtocol.carBodyCheckerRole, carFactoryProtocol, 0, subsCarFactory, [carBodyChecker, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const initialPayload = { parts: [] }
  const machine = createMachineRunnerBT(app, tags, s0Adapted, initialPayload, carBodyCheckerAdapted)
  printState(carBodyCheckerAdapted.machineName, s0Adapted.mechanism.name, initialPayload)
  console.log(chalk.bgBlack.red.dim`    carBody!`);

  for await (const state of machine) {
    if (state.isLike(s0)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s0)) {
          const currentState = state.cast()
          if (currentState.payload.parts.length == NUMBER_OF_CAR_PARTS) {
            console.log()
            stateAfterTimeOut?.cast().commands()?.carBodyDone()
          }

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