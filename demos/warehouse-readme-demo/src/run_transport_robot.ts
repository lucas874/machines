import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { manifest, TransportOrder, printState, getRandomInt, machineRunnerProtoName } from './protocol'
import { randomUUID } from "crypto";
import { Auction, DoIt, Done, initialTransportAdapted, Score, transportAdapted } from './transport_robot';

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = TransportOrder.tagWithEntityId(machineRunnerProtoName)
    const initialPayload = { robot: randomUUID().slice(0, 8) }
    const transportRobot = createMachineRunnerBT(app, tags, initialTransportAdapted, initialPayload, transportAdapted)
    let IamWinner = false
    const bestRobot = (scores: Score[]) => scores.reduce((best, current) => current.delay <= best.delay ? current : best).robot
    printState(transportAdapted.machineName, initialTransportAdapted.mechanism.name, initialPayload)
    for await (const state of transportRobot) {
        if (state.isLike(Auction)) {
            const auction = state.cast()
            if (!auction.payload.scores.find((s) => s.robot === auction.payload.robot)) {
                auction.commands()?.bid(getRandomInt(1, 50))
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
            if (!IamWinner) { console.log("Final state reached, press CTRL + D to quit."); console.log(); break }
            setTimeout(() => {
            const stateAfterTimeOut = transportRobot.get()
                if (stateAfterTimeOut?.isLike(DoIt)) {
                    console.log()
                    stateAfterTimeOut?.cast().commands()?.deliver()
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