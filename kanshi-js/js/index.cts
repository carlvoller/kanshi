// This module is the CJS entry point for the library.

// The Rust addon.
import * as addon from "./load.cjs";

// Use this declaration to assign types to the addon's exports,
// which otherwise by default are `any`.
declare module "./load.cjs" {
  function kanshiNew(opts: KanshiOptions): any;
  function kanshiWatch(dir: string): Promise<undefined>;
  function kanshiStart(callback: KanshiCallback): Promise<undefined>;
  function kanshiClose(): boolean;
}

type KanshiEventTypes =
  | "create"
  | "delete"
  | "modify"
  | "moved_to"
  | "moved_from"
  | "move"
  | "unknown";

interface KanshiEvent {
  eventType: KanshiEventTypes;
  target?: {
    /// Only set if eventType == "moved_from"
    previousPath?: string;
    /// Only set if eventType == "moved_to"
    newPath?: string;
    path: string;
    kind: "directory" | "file";
  };
}

interface KanshiOptions {
  forceEngine?: string
}

type KanshiCallback = (event: KanshiEvent) => void;

class Kanshi {
  #kanshi: any;
  #callbacks: Set<KanshiCallback>;
  #masterCallback: KanshiCallback;

  constructor(opts?: KanshiOptions) {
    this.#kanshi = addon.kanshiNew(!opts ? {} : opts);
    this.#callbacks = new Set();
    this.#masterCallback = (event) => {
      this.#callbacks.forEach((cb) => cb(event));
    };
    this.#masterCallback.bind(this);
  }

  async watch(dir: string): Promise<undefined> {
    return addon.kanshiWatch.call(this.#kanshi, dir);
  }

  onEvent(callback: KanshiCallback): () => void {
    this.#callbacks.add(callback);
    return () => this.#callbacks.delete(callback);
  }

  async start(): Promise<undefined> {
    return addon.kanshiStart.call(this.#kanshi, this.#masterCallback);
  }

  close(): boolean {
    return addon.kanshiClose.call(this.#kanshi);
  }
}

export default Kanshi;
export type { KanshiEvent, KanshiOptions, KanshiCallback, KanshiEventTypes };
