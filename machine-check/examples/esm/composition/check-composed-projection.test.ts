import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType,  getWWFSub, composeSubs, ResultData, Subscriptions, checkComposedProjection } from '../../..'
import { Events, Composition } from './car-factory-protos.js'
import { CompositionInputVec } from '../../../lib/cjs/index.js'

export namespace WeakWellFormed {
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

export namespace NotWeakWellFormed {
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

const result_subscriptions1: ResultData = getWWFSub(G1)
const result_subscriptions2: ResultData = getWWFSub(G2)

describe('extended subscriptions', () => {
  it('subscription1 should be ok', () => {
    expect(result_subscriptions1.type).toBe('OK')
  })

  it('subscription1 should be ok', () => {
    expect(result_subscriptions2.type).toBe('OK')
  })
})

if (result_subscriptions1.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions1: Subscriptions = JSON.parse(result_subscriptions1.data)

if (result_subscriptions2.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions2: Subscriptions = JSON.parse(result_subscriptions2.data)

const composition_input: CompositionInputVec = [{protocol: G1, subscription: subscriptions1, interface: null}, {protocol: G2, subscription: subscriptions2, interface: "T"}]

const result_subscriptions: ResultData = composeSubs(composition_input)
if (result_subscriptions.type === 'ERROR') throw new Error('error getting subscription')
const subscriptions: Subscriptions = JSON.parse(result_subscriptions.data)

describe('checkComposedProjection', () => {
  describe('weak-wellformed', () => {
    it('should match T', () => {
      expect(
        checkComposedProjection(
          composition_input,
          subscriptions,
          'T',
          WeakWellFormed.T.machine.createJSONForAnalysis(WeakWellFormed.T.S00)
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
          composition_input,
          subscriptions,
          'T',
          NotWeakWellFormed.T.machine.createJSONForAnalysis(NotWeakWellFormed.T.S00)
        ),
      ).toEqual({
        type: 'ERROR',
        errors: ["missing transition deliver/part in state S21 (from reference state { 2 } || { 1 })"]
      })
    })
  })
})
