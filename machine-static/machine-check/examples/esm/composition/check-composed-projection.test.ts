import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType, Subscriptions, DataResult, InterfacingProtocols, checkComposedProjection, overapproxWFSubscriptions} from '../../..'
import { Events, Composition } from './car-factory-protos.js'


export namespace OkMachine {
    export namespace T {
        export const machine = Composition.makeMachine('T')
        export const S00 = machine
            .designEmpty('S00')
            .command('request', [Events.partID], () => [{}])
            .finish()
        export const S30 = machine.designEmpty('S30').finish()
        export const S11 = machine.designEmpty('S11').finish()
        export const S21 = machine
            .designEmpty('S21')
            .command('deliver', [Events.part], () => [{}])
            .finish()
        export const S03 = machine.designEmpty('S03').finish()
        export const S33 = machine.designEmpty('S33').finish()

        S00.react([Events.partID], S11, () => undefined)
        S00.react([Events.time], S30, () => undefined)
        S11.react([Events.position], S21, () => undefined)
        S21.react([Events.part], S03, () => undefined)
        S03.react([Events.time], S33, () => undefined)
    }
}

export namespace NotOkMachine {
    export namespace T {
        export const machine = Composition.makeMachine('T')
        export const S00 = machine
            .designEmpty('S00')
            .command('request', [Events.partID], () => [{}])
            .finish()
        export const S30 = machine.designEmpty('S30').finish()
        export const S11 = machine.designEmpty('S11').finish()
        export const S21 = machine.designEmpty('S21').finish()
        export const S03 = machine.designEmpty('S03').finish()

        S00.react([Events.partID], S11, () => undefined)
        S00.react([Events.time], S30, () => undefined)
        S11.react([Events.position], S21, () => undefined)
        S21.react([Events.part], S03, () => undefined)
        S03.react([Events.time], S30, () => undefined)
    }
}

const G1: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {
      source: '0',
      target: '1',
      label: { cmd: 'request', role: 'T', logType: [Events.partID.type] },
    },
    {
      source: '1',
      target: '2',
      label: { cmd: 'get', role: 'FL', logType: [Events.position.type] },
    },
    {
      source: '2',
      target: '0',
      label: { cmd: 'deliver', role: 'T', logType: [Events.part.type] },
    },
    {
      source: '0',
      target: '3',
      label: { cmd: 'close', role: 'D', logType: [Events.time.type] },
    },
  ],
}

const G2: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {
      source: '0',
      target: '1',
      label: { cmd: 'request', role: 'T', logType: [Events.partID.type] },
    },
    {
      source: '1',
      target: '2',
      label: { cmd: 'deliver', role: 'T', logType: [Events.part.type] },
    },
    {
      source: '2',
      target: '3',
      label: { cmd: 'build', role: 'F', logType: [Events.car.type] },
    },
  ],
}

const interfacing_swarms: InterfacingProtocols = [G1, G2]
const overapprox_result_subscriptions: DataResult<Subscriptions> = overapproxWFSubscriptions(interfacing_swarms, {}, "Coarse")

describe('subscriptions', () => {
  it('overapproximation should be ok', () => {
    expect(overapprox_result_subscriptions.type).toBe('OK')
  })
})

if (overapprox_result_subscriptions.type === 'ERROR') throw new Error('error getting subscription')
const overapprox_subscriptions: Subscriptions = overapprox_result_subscriptions.data

describe('checkComposedProjection', () => {
  describe('weak-wellformed', () => {
    it('should match T', () => {
      expect(
        checkComposedProjection(
          interfacing_swarms,
          overapprox_subscriptions,
          'T',
          OkMachine.T.machine.createJSONForAnalysis(OkMachine.T.S00)
        ),
      ).toEqual({
        type: 'OK',
      })
    })
  })

  describe('not weak-wellformed', () => {
    it('should match T', () => {
      expect(
        checkComposedProjection(
          interfacing_swarms,
          overapprox_subscriptions,
          'T',
          NotOkMachine.T.machine.createJSONForAnalysis(NotOkMachine.T.S00)
        ),
      ).toEqual({
        type: 'ERROR',
        errors: ["missing transition deliver/part in state S21 (from reference state 2 || 1)"]
      })
    })
  })
})
