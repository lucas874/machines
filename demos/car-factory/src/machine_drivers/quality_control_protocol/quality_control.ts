import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, QualityControlProtocol, getArgs, manifestFromArgs } from '../../protocol.js'
import { qualityControl, s0, s1 } from '../../machines/quality_control_protocol/quality_control.js'

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [qualityControlAdapted, s0Adapted] = Composition.adaptMachine(QualityControlProtocol.qualityControlRole, carFactoryProtocol, 6, subsCarFactory, [qualityControl, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, qualityControlAdapted)
  printState(qualityControlAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)
            && stateAfterTimeOut?.payload.wheelsChecked
            && stateAfterTimeOut?.payload.windowsChecked) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.checkCar()
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