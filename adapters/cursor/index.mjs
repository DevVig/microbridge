#!/usr/bin/env node
// Compatibility entrypoint for local hook testing. Cursor installations use
// the managed plugin in `.cursor-plugin/plugin.json` and `hooks/hooks.json`.
import "./hooks/microbridge-event.mjs";
