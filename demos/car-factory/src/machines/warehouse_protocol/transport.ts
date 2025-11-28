import { Events, Composition, WarehouseProtocol } from '../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

type InitialPayload = { id: string }
export type Score = { transportId: string, delay: number }
type AuctionPayload = { id: string, item: string, to: string, scores: Score[] }
type SelectedPayload = { id: string, item: string, to: string, winner: string }

// Using the machine runner DSL an implmentation of body assembler in the steel press protocol:
export const transport = Composition.makeMachine(WarehouseProtocol.transportRole)
export const s0 = transport.designState('s0').withPayload<InitialPayload>().finish()
export const s1 = transport.designState('s1').withPayload<AuctionPayload>()
    .command(WarehouseProtocol.cmdBid, [Events.bid], (ctx, delay: number) =>
        [{transportId: ctx.self.id, delay}])
    .command(WarehouseProtocol.cmdSelect, [Events.selected], (_, winner: string) =>
        [{winnerTransport: winner}])
    .finish()
export const s2 = transport.designState('s2').withPayload<SelectedPayload>()
    .command(WarehouseProtocol.cmdNeedGuidance, [Events.requestGuidance], (ctx) =>
        [Events.requestGuidance.make({ item: ctx.self.item, to: ctx.self.to })])
    .command(WarehouseProtocol.cmdSmartPickup, [Events.itemPickupSmart], (ctx) =>
        [{item: ctx.self.item, to: ctx.self.to}])
    .finish()
export const s3 = transport.designState('s3').withPayload<SelectedPayload>().finish()
export const s4 = transport.designState('s4').withPayload<SelectedPayload>()
    .command(WarehouseProtocol.cmdBasicPickup, [Events.itemPickupBasic], (ctx) =>
        [{item: ctx.self.item, to: ctx.self.to}])
    .finish()
export const s5 = transport.designState('s5').withPayload<SelectedPayload>()
    .command(WarehouseProtocol.cmdHandover, [Events.handover], (ctx) =>
        [{item: ctx.self.item, to: ctx.self.to}])
    .finish()

s0.react([Events.itemRequest], s1, (ctx, event) => {
    return s1.make({id: ctx.self.id, item: event.payload.item, to: event.payload.to, scores: [] })
})
s1.react([Events.bid], s1, (ctx, event) => {
    ctx.self.scores.push({transportId: event.payload.transportId, delay: event.payload.delay});
    return ctx.self
})
s1.react([Events.selected], s2, (ctx, event) => {
    return s2.make( { id: ctx.self.id, item: ctx.self.item, to: ctx.self.to, winner: event.payload.winnerTransport } )
})
s2.react([Events.requestGuidance], s3, (ctx) => { return ctx.self })
s2.react([Events.itemPickupSmart], s5, (ctx) => { return ctx.self })
s3.react([Events.giveGuidance], s4, (ctx) => { return ctx.self })
s4.react([Events.itemPickupBasic], s5, (ctx) => { return ctx.self })
s5.react([Events.handover], s0, (ctx) => { return s0.make({id: ctx.self.id}) })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([WarehouseProtocol.protocol], WarehouseProtocol.subscriptions, WarehouseProtocol.transportRole, transport.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))