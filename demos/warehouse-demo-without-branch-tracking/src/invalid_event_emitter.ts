import { Actyx } from '@actyx/sdk'
import { Events, manifest, Composition  } from './warehouse_protocol'

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-1')
    while(true) {
        await new Promise(f => setTimeout(f, 5000));
        await app.publish(tags.apply(Events.closingTime.makeBT({timeOfDay: new Date().toLocaleString()}, "invalidPointer")))
        console.log('Publishing time event with invalid lbj pointer')
    }
    app.dispose()
}

main()