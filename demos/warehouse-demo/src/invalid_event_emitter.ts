import { Actyx } from '@actyx/sdk'
import { Events, manifest, Composition  } from './protocol'

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse')
    while(true) {
        await new Promise(f => setTimeout(f, 5000));
        await app.publish(tags.apply(Events.time.makeBT({timeOfDay: new Date().toLocaleString()}, "invalidPointer")))
        console.log('Publishing time event with invalid lbj pointer')
    }
    app.dispose()
}

main()