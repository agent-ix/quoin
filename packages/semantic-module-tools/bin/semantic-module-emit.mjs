#!/usr/bin/env node
import { main } from "../src/cli.mjs";

process.exitCode = main(process.argv.slice(2));
