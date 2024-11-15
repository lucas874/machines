import { describe, expect, it } from '@jest/globals'
import { SwarmProtocolType, checkSwarmProtocol, getWWFSub, checkWWFSwarmProtocol, ResultData, Subscriptions } from '../../..'
import { Events } from './car-factory-protos.js'

/*
 * example from CoPLaWS slides by Florian Furbach
 * protocols are wwf but not wf under the generated subscriptions.
 *
 */

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
      target: '0',
      label: { cmd: 'build', role: 'F', logType: [Events.car.type] },
    },
  ],
}

const G3: SwarmProtocolType = {
  initial: '0',
  transitions: [
    {
      source: '0',
      target: '1',
      label: { cmd: 'build', role: 'F', logType: [Events.report.type] },
    },
    {
      source: '1',
      target: '2',
      label: { cmd: 'test', role: 'TR', logType: [Events.report.type] },
    },
    {
      source: '2',
      target: '3',
      label: { cmd: 'accept', role: 'QCR', logType: [Events.ok.type] },
    },
    {
      source: '2',
      target: '3',
      label: { cmd: 'reject', role: 'QCR', logType: [Events.notOk.type] },
    },
  ],
}

const subscriptions1 = {
    T: [
      Events.partID.type,
      Events.time.type,
      Events.position.type,
    ],
    D: [
      Events.partID.type,
      Events.time.type,
    ],
    FL: [
      Events.partID.type,
      Events.position.type,
    ],
}


const result_subscriptions2: ResultData = getWWFSub(G2)
const result_subscriptions3: ResultData = getWWFSub(G3)

describe('extended subscriptions', () => {
  it('subscription1 should be ok', () => {
    expect(result_subscriptions2).toEqual({
      type: 'ERROR',
      errors: [
        "state 0 can not reach terminal node",
        "state 1 can not reach terminal node",
        "state 2 can not reach terminal node",
      ]
    })
  })

  it('subscription3 should be ok', () => {
    expect(result_subscriptions3).toEqual({
      type: 'ERROR',
      errors: [
        "guard event type report appears in transitions from multiple states",
        "event type report emitted by command in transition (0)--[build@F<report>]-->(1) and command in transition (1)--[test@TR<report>]-->(2)",
      ]
    })
  })
})

describe('checkWWFSwarmProtocol G1', () => {
  it('should catch not well-formed protocol', () => {
    expect(checkWWFSwarmProtocol(G1, subscriptions1)).toEqual({
      type: 'ERROR',
      errors: [
        "role FL does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
        "active role does not subscribe to any of its emitted event types in transition (2)--[deliver@T<part>]-->(0)",
        "subsequently active role D does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
        "subsequently active role T does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
      ]
    })
  })
})