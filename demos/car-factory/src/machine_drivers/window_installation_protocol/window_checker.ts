import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, WindowInstallationProtocol, getArgs, manifestFromArgs } from '../../protocol.js'
import { s0, s1, windowChecker } from '../../machines/window_installation_protocol/window_checker.js'

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [windowCheckerAdapted, s0Adapted] = Composition.adaptMachine(WindowInstallationProtocol.windowCheckerRole, carFactoryProtocol, 5, subsCarFactory, [windowChecker, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, windowCheckerAdapted)
  printState(windowCheckerAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          console.log()
          const shape = stateAfterTimeOut.payload.shape
          const numWindows = stateAfterTimeOut.payload.numWindows
          if (  shape === "truck" && numWindows == 3 ||
                shape === "sedan" && numWindows == 4) {
            stateAfterTimeOut?.cast().commands()?.windowsDone()
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