"use strict";

const assert = require("assert");

const kanshi = require("..");

describe("should print hello", () => {
  it("should say hello", () => {
    console.log(kanshi.greeting());
  })
})