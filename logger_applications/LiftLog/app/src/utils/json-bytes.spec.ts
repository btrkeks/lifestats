import { describe, expect, it } from 'vitest';
import { fromJsonBytes, toJsonBytes } from '@/utils/json-bytes';

describe('toJsonBytes / fromJsonBytes', () => {
  it('round-trips a simple object', () => {
    const obj = { a: 1, b: 'hello' };
    expect(fromJsonBytes(toJsonBytes(obj))).toEqual(obj);
  });

  it('round-trips an array', () => {
    const arr = [1, 'two', true, null];
    expect(fromJsonBytes(toJsonBytes(arr))).toEqual(arr);
  });

  it('round-trips a nested object', () => {
    const obj = { x: { y: { z: 42 } } };
    expect(fromJsonBytes(toJsonBytes(obj))).toEqual(obj);
  });

  it('round-trips a string', () => {
    expect(fromJsonBytes(toJsonBytes('hello'))).toBe('hello');
  });

  it('round-trips a number', () => {
    expect(fromJsonBytes(toJsonBytes(123.456))).toBe(123.456);
  });

  it('round-trips null', () => {
    expect(fromJsonBytes(toJsonBytes(null))).toBeNull();
  });

  it('round-trips a boolean', () => {
    expect(fromJsonBytes(toJsonBytes(true))).toBe(true);
  });

  it('encodes to UTF-8 bytes', () => {
    const result = toJsonBytes({ a: 1 });
    expect(new TextDecoder().decode(result)).toBe('{"a":1}');
  });

  it('handles unicode correctly', () => {
    const obj = { text: '日本語 émoji 🎉' };
    expect(fromJsonBytes(toJsonBytes(obj))).toEqual(obj);
  });
});
