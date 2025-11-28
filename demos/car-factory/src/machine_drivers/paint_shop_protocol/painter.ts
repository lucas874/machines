import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, getRandomInt, PaintShopProtocol, getArgs, manifestFromArgs } from '../../protocol.js'
import { painter, s0, s1 } from '../../machines/paint_shop_protocol/painter.js';

// A car can have one of these colors
const colors = ["red", "blue", "green"]

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [painterAdapted, s0Adapted] = Composition.adaptMachine(PaintShopProtocol.painterRole, carFactoryProtocol, 1, subsCarFactory, [painter, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, painterAdapted)
  printState(painterAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.applyPaint(colors[Math.floor(Math.random() * colors.length)])
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