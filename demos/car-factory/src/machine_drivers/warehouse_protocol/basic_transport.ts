import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Composition, carFactoryProtocol, subsCarFactory, printState, WarehouseProtocol, getRandomInt, getArgs, manifestFromArgs } from '../../protocol.js'
import { randomUUID } from 'crypto';
import { s0, s1, s2, s4, s5, transport, type Score } from '../../machines/warehouse_protocol/transport.js';

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [transportAdapted, s0Adapted] = Composition.adaptMachine(WarehouseProtocol.transportRole, carFactoryProtocol, 3, subsCarFactory, [transport, s0], true).data!

// Run the adapted machine
async function main() {
  const argv = getArgs()
  const app = await Actyx.of(manifestFromArgs(argv))
  const tags = Composition.tagWithEntityId(argv.displayName)
  const initialPayload = { id: randomUUID().slice(0, 8) }
  const bestTransport = (scores: Score[]) => scores.reduce((best, current) => current.delay <= best.delay ? current : best).transportId
  const machine = createMachineRunnerBT(app, tags, s0Adapted, initialPayload, transportAdapted)
  printState(transportAdapted.machineName, s0Adapted.mechanism.name, initialPayload)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      const auctionState = state.cast()
      if (!auctionState.payload.scores.find((score) => score.transportId === auctionState.payload.id)) {
        console.log()
        auctionState.commands()?.bid(getRandomInt(1, 50))
        setTimeout(() => {
              const stateAfterTimeOut = machine.get()
              if (stateAfterTimeOut?.isLike(s1)) {
                  console.log()
                  stateAfterTimeOut?.cast().commands()?.select(bestTransport(stateAfterTimeOut.payload.scores))
              }
        }, 3000)
      }
    } else if (state.isLike(s2)) {
      // Break out of loop if this transport did not win the auction
      const IamWinner = state.payload.id === state.payload.winner
      if (!IamWinner) { console.log("Final state reached, press CTRL + D to quit."); break }
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s2)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.needGuidance()
        }
      }, 1000)
    } else if (state.isLike(s4)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s4)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.basicPickup()
        }
      }, 1000)
    } else if (state.isLike(s5)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s5)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.handover()
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