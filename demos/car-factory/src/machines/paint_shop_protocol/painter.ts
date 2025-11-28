import { Events, Composition, PaintShopProtocol } from './../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

// Using the machine runner DSL an implmentation of body assembler in the steel press protocol:
export const painter = Composition.makeMachine(PaintShopProtocol.painterRole)
export const s0 = painter.designEmpty('s0').finish()
export const s1 = painter.designState('s1')
    .withPayload<{shape: string}>()
    .command(PaintShopProtocol.cmdPaintBody, [Events.paintedCarBody], (ctx, color: string) => {
        return [Events.paintedCarBody.make({ shape: ctx.self.shape, color: color })]
    })
    .finish()
export const s2 = painter.designEmpty('s2')
    .finish()

s0.react([Events.carBody], s1, (_, event) => { return s1.make({ shape: event.payload.shape }) })
s1.react([Events.paintedCarBody], s2, (_) => { return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([PaintShopProtocol.protocol], PaintShopProtocol.subscriptions, PaintShopProtocol.painterRole, painter.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))