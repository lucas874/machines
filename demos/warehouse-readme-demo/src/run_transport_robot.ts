import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { manifest, Composition, printState, getRandomInt } from './protocol'
import { randomUUID } from "crypto";
import { Auction, DoIt, initialAdapted, Score, transportAdapted } from './transport_robot';

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