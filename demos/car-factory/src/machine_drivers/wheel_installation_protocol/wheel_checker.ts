import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, WheelInstallationProtocol, getArgs, manifestFromArgs } from '../../protocol.js'
import { s0, s1, wheelChecker } from '../../machines/wheel_installation_protocol/wheel_checker.js'

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [wheelCheckerAdapted, s0Adapted] = Composition.adaptMachine(WheelInstallationProtocol.wheelCheckerRole, carFactoryProtocol, 4, subsCarFactory, [wheelChecker, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, wheelCheckerAdapted)
  printState(wheelCheckerAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          console.log()
          const shape = stateAfterTimeOut.payload.shape
          const numWheels = stateAfterTimeOut.payload.numWheels
          if (  shape === "truck" && numWheels == 6 ||
                shape === "sedan" && numWheels == 4) {
            stateAfterTimeOut?.cast().commands()?.wheelsDone()
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