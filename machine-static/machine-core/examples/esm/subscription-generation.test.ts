import { describe, expect, it } from '@jest/globals'
import { WarehouseFactory } from './proto.js'
import { exactWFSubscriptions, Subscriptions, DataResult, overapproxWFSubscriptions, Granularity } from '../..'

/*
 * This file tests subscription generation for the compositition described in proto.ts.
 */

const subscriptionsExact = {
  T: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime, WarehouseFactory.eventTypePos],
  FL: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypeClosingTime, WarehouseFactory.eventTypePos],
  D: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime],
  R: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime, WarehouseFactory.eventTypeCar] 
}

const subscriptionsOverapprox = {
  T: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime, WarehouseFactory.eventTypePos],
  FL: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime, WarehouseFactory.eventTypePos],
  D: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime],
  R: [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime, WarehouseFactory.eventTypeCar] 
}

describe('generate subscription for warehouse || factory', () => {
  it('exact', () => {
    const result: DataResult<Subscriptions> = exactWFSubscriptions(WarehouseFactory.protocols, {})
    expect(result.type).toBe('OK')
    if (result.type === 'ERROR') throw new Error('expected subscription generation result to be ok')
    expect(result.data[WarehouseFactory.roleT].sort()).toEqual(subscriptionsExact[WarehouseFactory.roleT].sort())
    expect(result.data[WarehouseFactory.roleFL].sort()).toEqual(subscriptionsExact[WarehouseFactory.roleFL].sort())
    expect(result.data[WarehouseFactory.roleD].sort()).toEqual(subscriptionsExact[WarehouseFactory.roleD].sort())
    expect(result.data[WarehouseFactory.roleR].sort()).toEqual(subscriptionsExact[WarehouseFactory.roleR].sort())
  })
  it('overapproximation', () => {
    const result: DataResult<Subscriptions> = overapproxWFSubscriptions(WarehouseFactory.protocols, {}, 'TwoStep')
    expect(result.type).toBe('OK')
    if (result.type === 'ERROR') throw new Error('expected subscription generation result to be ok')
    expect(result.data[WarehouseFactory.roleT].sort()).toEqual(subscriptionsOverapprox[WarehouseFactory.roleT].sort())
    expect(result.data[WarehouseFactory.roleFL].sort()).toEqual(subscriptionsOverapprox[WarehouseFactory.roleFL].sort())
    expect(result.data[WarehouseFactory.roleD].sort()).toEqual(subscriptionsOverapprox[WarehouseFactory.roleD].sort())
    expect(result.data[WarehouseFactory.roleR].sort()).toEqual(subscriptionsOverapprox[WarehouseFactory.roleR].sort())
  })
})
