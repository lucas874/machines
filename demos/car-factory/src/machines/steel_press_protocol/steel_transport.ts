import { Events, Composition, SteelPressProtocol } from './../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

type SteelTransportPayload = { steelRollsDelivered: number }

// Using the machine runner DSL an implmentation of steel transport in the steel press protocol:
export const steelTransport = Composition.makeMachine(SteelPressProtocol.steelTransportRole)
export const s0 = steelTransport.designState('s0')
    .withPayload<SteelTransportPayload>()
    .command(SteelPressProtocol.cmdPickUpSteel, [Events.steelRoll], () => {
    return [Events.steelRoll.make({})]
  })
  .finish()
export const s1 = steelTransport.designState('s1').withPayload<SteelTransportPayload>().finish()
export const s2 = steelTransport.designState('s2').withPayload<SteelTransportPayload>().finish()

s0.react([Events.steelRoll], s1, (ctx) => { return s1.make({ steelRollsDelivered: ctx.self.steelRollsDelivered + 1 }) })
s1.react([Events.partialCarBody], s0, (ctx) => { return s0.make(ctx.self) })
s0.react([Events.carBody], s2, (ctx) => { return s2.make(ctx.self) })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([SteelPressProtocol.protocol], SteelPressProtocol.subscriptions, SteelPressProtocol.steelTransportRole, steelTransport.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))