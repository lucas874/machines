import { Events, Composition, subsWarehouse, warehouseProtocol, factoryProtocol, subscriptions } from './protocol'
import { checkComposedProjection } from '@actyx/machine-check';

export const TransportOrderForRobot = Composition.makeMachine('robot')

export type Score = { robot: string; delay: number }
export type AuctionPayload =
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

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
export const [transportAdapted, initialAdapted] = Composition.adaptMachine('robot', [warehouseProtocol, factoryProtocol], 0, subscriptions, [TransportOrderForRobot, Initial], true).data!
