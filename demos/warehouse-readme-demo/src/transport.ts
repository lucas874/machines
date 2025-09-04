import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, printState, subsWarehouse, warehouseProtocol, factoryProtocol, subscriptions, getRandomInt } from './protocol'
import { randomUUID } from "crypto";
import { checkComposedProjection } from '@actyx/machine-check';

const TransportOrderForRobot = Composition.makeMachine('robot')

type Score = { robot: string; delay: number }
type AuctionPayload =
  { id: string; from: string; to: string; robot: string; scores: Score[] }

export const Initial = TransportOrderForRobot.designState('Initial')
  .withPayload<{ robot: string }>()
  .finish()
export const Auction = TransportOrderForRobot.designState('Auction')
  .withPayload<AuctionPayload>()
  .command('bid', [Events.bid], (ctx, delay: number) =>
                         [{ robot: ctx.self.robot, delay, id: ctx.self.id }])
  .command('select', [Events.selected], (ctx, winner: string) => [{ winner, id: ctx.self.id}])
  .finish()
export const DoIt = TransportOrderForRobot.designState('DoIt')
  .withPayload<{ robot: string; winner: string, id: string }>()
  .command('deliver', [Events.deliver], (ctx) => [{ id: ctx.self.id }])
  .finish()
export const Done = TransportOrderForRobot.designEmpty('Done').finish()

// ingest the request from the `warehouse`
Initial.react([Events.request], Auction, (ctx, r) => ({
  id: r.payload.id,
  from: r.payload.from,
  to: r.payload.to,
  robot: ctx.self.robot,
  scores: []
}))

// accumulate bids from all `robot`
Auction.react([Events.bid], Auction, (ctx, b) => {
  ctx.self.scores.push({robot: b.payload.robot, delay: b.payload.delay})
  return ctx.self
})

// end the auction when a selection has happened
Auction.react([Events.selected], DoIt, (ctx, s) =>
  ({ robot: ctx.self.robot, winner: s.payload.winner, id: ctx.self.id }))

// go to the final state
DoIt.react([Events.deliver], Done, (_ctx) => {[]})

// Check that the machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([warehouseProtocol], subsWarehouse, "T", TransportOrderForRobot.createJSONForAnalysis(Initial))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [transportAdapted, initialAdapted] = Composition.adaptMachine('T', [warehouseProtocol, factoryProtocol], 0, subscriptions, [TransportOrderForRobot, Initial], true).data!

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory')
    const transportRobot = createMachineRunnerBT(app, tags, initialAdapted, { robot: randomUUID() }, transportAdapted)
    let IamWinner = false
    const bestRobot = (scores: Score[]) => scores.reduce((best, current) => current.delay <= best.delay ? current : best).robot
    printState(transportAdapted.machineName, initialAdapted.mechanism.name, undefined)
    for await (const state of transportRobot) {
    if (state.isLike(Auction)) {
        const auction = state.cast()
        if (!auction.payload.scores.find((s) => s.robot === auction.payload.robot)) {

            auction.commands()?.bid(getRandomInt(1, 10))
            setTimeout(() => {
                const stateAfterTimeOut = transportRobot.get()
                if (stateAfterTimeOut?.isLike(Auction)) {
                    stateAfterTimeOut?.cast().commands()?.select(bestRobot(auction.payload.scores))
                }
            }, 3000)
        }
    } else if (state.isLike(DoIt)) {
        const assigned = state.cast()
        IamWinner = assigned.payload.winner === assigned.payload.robot
        if (!IamWinner) break
        assigned.commands()?.deliver()
    }
    }
}

main()
