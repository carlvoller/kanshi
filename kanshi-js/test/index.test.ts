"use strict";

import assert from "assert";
import fs from "node:fs";
import Kanshi, { KanshiEvent } from "..";

describe("#kanshiSuccess", () => {
  describe("Create a Kanshi Instance", () => {
    before(() => {
      try {
        fs.rmSync("./test_dir", { recursive: true, force: true });
      } catch {}

      fs.mkdirSync("./test_dir", {});
    });

    after(() => {
      try {
        fs.rmSync("./test_dir", { recursive: true, force: true });
      } catch {}
    });

    const kan = new Kanshi();
    assert(kan instanceof Kanshi);

    it("Fails to watches a non existent directory", async () => {
      assert.rejects(async () => await kan.watch("./fake_dir"), /ENOENT/);
    });

    it("Watches directory successfully", async () => {
      await kan.watch("./test_dir");
    });

    it("Register Callback, Start Listener and Receives a Create Event", async () => {
      const start = kan.start();

      const waitForEvent = new Promise<void>((resolve, reject) => {
        kan.onEvent((event: KanshiEvent) => {
          if (event.eventType === "create") resolve();
          else reject(`Received wrong event type: ${event.eventType}`);
        });
      });

      fs.writeFileSync("./test_dir/hello_world.txt", "testing...");

      return Promise.all([start, waitForEvent]);
    });

    it("Fails to Start Listener again", async () => {
      assert.rejects(async () => kan.start(), /already started/);
    });

    it("Closes successfully", () => {
      assert(kan.close());
    });
  });
});
