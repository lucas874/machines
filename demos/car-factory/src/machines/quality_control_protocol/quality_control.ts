import { Events, Composition, QualityControlProtocol } from '../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

type CheckCarPayload = { shape: string, color: string, engine: string, numWheels: number, numWindows: number,  wheelsChecked: boolean, windowsChecked: boolean}

export const qualityControl = Composition.makeMachine(QualityControlProtocol.qualityControlRole)
export const s0 = qualityControl.designEmpty('s0').finish()
export const s1 = qualityControl.designState('s1')
    .withPayload<CheckCarPayload>()
    .command(QualityControlProtocol.cmdCheckCar, [Events.finishedCar], (ctx) => {
        const okTruck = ctx.self.shape === "truck"
            && ctx.self.engine === "truckEngine"
        const okSedan = ctx.self.shape === "sedan"
            && ctx.self.engine === "basicEngine"
        const okColor = ctx.self.color != "" && ctx.self.color != undefined
        const isOk = (okTruck || okSedan)
            && okColor
            && ctx.self.wheelsChecked
            && ctx.self.windowsChecked
        const { shape, color, engine, numWheels, numWindows } = ctx.self
        return [Events.finishedCar.make({ shape, color, engine, numWheels, numWindows, isOk})]
    })
    .finish()
export const s2 = qualityControl.designEmpty('s2').finish()

s0.react([Events.engineChecked], s1, (_, event) => {
    const { shape, color, engine } = event.payload;
    return s1.make({
        shape, color, engine,
        numWheels: 0,
        numWindows: 0,
        wheelsChecked: false,
        windowsChecked: false
    }
    )
})
s1.react([Events.wheelsDone], s1, (ctx, event) => {
    const okWheels =ctx.self.shape === "sedan" && event.payload.numWheels == 4
        || ctx.self.shape === "truck" && event.payload.numWheels == 6
    return s1.make({ ...ctx.self, numWheels: event.payload.numWheels, wheelsChecked: okWheels })
})
s1.react([Events.windowsDone], s1, (ctx, event) => {
    const okWindows = ctx.self.shape === "sedan" && event.payload.numWindows == 4
        || ctx.self.shape === "truck" && event.payload.numWindows == 3
    return s1.make({ ...ctx.self, numWindows: event.payload.numWindows, windowsChecked: okWindows })
})
s1.react([Events.finishedCar], s2, () => s2.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([QualityControlProtocol.protocol], QualityControlProtocol.subscriptions, QualityControlProtocol.qualityControlRole, qualityControl.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))