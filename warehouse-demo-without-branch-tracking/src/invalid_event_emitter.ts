import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, all_projections, getRandomInt  } from './warehouse_protocol'
import { projectCombineMachines, checkComposedProjection } from '@actyx/machine-check'

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    while(true) {
        //await new Promise(f => setTimeout(f, 2000));
        //await app.publish(tags.apply(Events.partID.makeBT({id: "tire"}, "invalidPointer")))
        //console.log('Publishing partID event with invalid lbj pointer')

        await new Promise(f => setTimeout(f, 5000));
        await app.publish(tags.apply(Events.time.makeBT({timeOfDay: new Date().toLocaleString()}, "invalidPointer")))
        console.log('Publishing time event with invalid lbj pointer')
    }
    app.dispose()
}

main()