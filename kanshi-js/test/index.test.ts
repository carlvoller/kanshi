"use strict";

import assert from "assert";
import fs from "node:fs";
import Kanshi, { KanshiEvent } from "..";

describe("#kanshiSuccess", () => {
  describe("Create a Kanshi Instance", () => {

    // after(() => {
    //   try {
    //     // fs.rmSync("./test_dir", { recursive: true, force: true });
    //   } catch {}
    // });

    const kan = new Kanshi();
    assert(kan instanceof Kanshi);

    it("Create test directory", () => {
      try {
        fs.rmSync("./test_dir", { recursive: true, force: true });
      } catch {}
      fs.mkdirSync("./test_dir", {});
    });

    it("Fails to watches a non existent directory", async () => {
      assert.rejects(async () => await kan.watch("./fake_dir"), /ENOENT/);
    });

    it("Watch directory and test events", async () => {
      return kan.watch("./test_dir").then(() => {
        setTimeout(() => {
          fs.writeFileSync("./test_dir/hello_world.txt", "testing...");
        }, 1000);

        setTimeout(() => {
          fs.renameSync("./test_dir/hello_world.txt", "./test_dir/bye.txt");
        }, 1500);
        
        setTimeout(() => {
          fs.rmSync("./test_dir/bye.txt");
        }, 2000);

        const start = kan.start();

        const waitForEvent = new Promise<void>((resolve, reject) => {
          let hasCreate = false;
          let hasMove = false;
          let hasDelete = false;
          kan.onEvent((event: KanshiEvent) => {
            if (event.eventType === "create") hasCreate = true;
            if (event.eventType === "moved_from") hasMove = true;
            if (event.eventType === "delete") hasDelete = true;
            if (hasCreate && hasDelete && hasMove) resolve();

            // else reject(`Received wrong event type: ${event.eventType}`);
          });
        });

        return waitForEvent;
      });
    });

    it("Fails to Start Listener again", async () => {
      assert.rejects(async () => await kan.start(), /already started/);
    });

    it("Closes successfully", () => {
      assert(kan.close());
      // assert(kan.close());
    });
  });
});
