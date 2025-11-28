import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, EngineInstallationProtocol, getArgs, manifestFromArgs } from '../../protocol.js'
import { engineChecker, s0, s1 } from '../../machines/engine_installation_protocol/engine_checker.js'

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [engineCheckerAdapted, s0Adapted] = Composition.adaptMachine(EngineInstallationProtocol.engineCheckerRole, carFactoryProtocol, 2, subsCarFactory, [engineChecker, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, engineCheckerAdapted)
  printState(engineCheckerAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.checkEngine()
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