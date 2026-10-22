// Deep "at least these fields": the server may send fields the UI's types leave out, but every
// field the UI declares must be there with the declared type. Used by the generated API samples.
export type Loose<T> = T extends readonly unknown[]
  ? { [K in keyof T]: Loose<T[K]> }
  : T extends object
    ? { [K in keyof T]: Loose<T[K]> } & { [extra: string]: unknown }
    : T;
