
declare type ObjectFromMap<M, T extends keyof any = "type">
    = { [K in keyof M]: M[K] & { [_ in T]: K } }[keyof M];

declare type ArrayFromMap<M, T extends boolean = true>
    = T extends true ? { [K in keyof M]: [type: K, ...M[K]] }[keyof M] : { [K in keyof M]: [...M[K]] }[keyof M];