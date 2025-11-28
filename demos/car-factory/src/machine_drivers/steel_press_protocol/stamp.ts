import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, getRandomInt, SteelPressProtocol, getArgs, manifestFromArgs } from '../../protocol.js'
import { s0, s1, stamp } from '../../machines/steel_press_protocol/stamp.js';

// Car parts that the stamp can produce

const getPart = (i: number): string => {
  switch (i) {
    case 0:
      return "frontFrame"
    case 1:
      return Math.random() >= 0.5 ? "loadBed" : "rearFrame"
    case 2:
      return "roof"
    default:
      return ""
  }
}
let index = 0

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [stampAdapted, s0Adapted] = Composition.adaptMachine(SteelPressProtocol.stampRole, carFactoryProtocol, 0, subsCarFactory, [stamp, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, stampAdapted)
  printState(stampAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.pressSteel(getPart(index))
          index = (index + 1) % 3
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