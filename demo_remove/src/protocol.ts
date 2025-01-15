import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'

export const manifest = {
  appId: 'com.example.tomato-robot',
  displayName: 'Tomato Robot',
  version: '1.0.0',
}

export namespace Events {
  export const HasWater = MachineEvent.design('HasWater').withoutPayload()
  export const NeedsWater = MachineEvent.design('NeedsWater').withoutPayload()
  export const Done = MachineEvent.design('Done').withoutPayload()
  export const All = [HasWater, NeedsWater, Done] as const
}

export const protocol = SwarmProtocol.make('wateringRobot', Events.All)