import { Actyx } from '@actyx/sdk'
import { Events, manifest, protocol } from './protocol'

async function main() {
  const app = await Actyx.of(manifest)
  const tags = protocol.tagWithEntityId('robot-1')

  await app.publish(tags.apply(Events.NeedsWater.make({})))
  console.log('Publishing NeedsWater')

  await app.publish(tags.apply(Events.HasWater.make({})))
  console.log('Publishing HasWater')

  app.dispose()
}

main()