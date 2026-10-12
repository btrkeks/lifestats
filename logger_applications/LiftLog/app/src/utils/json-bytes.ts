export function toJsonBytes(object: unknown): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(object));
}

export function fromJsonBytes<T>(jsonBytes: Uint8Array): T {
  return JSON.parse(new TextDecoder().decode(jsonBytes)) as T;
}
