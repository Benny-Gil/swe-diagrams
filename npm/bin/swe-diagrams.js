#!/usr/bin/env node
// Runs the swe-diagrams binary. The first run downloads it.
"use strict";

require("../lib/launcher.js").main(process.argv.slice(2));
