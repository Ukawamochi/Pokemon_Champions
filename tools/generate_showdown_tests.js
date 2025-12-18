#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");

require("ts-node").register({
  transpileOnly: true,
  compilerOptions: {
    module: "CommonJS",
    moduleResolution: "Node",
    esModuleInterop: true,
    target: "ES2019",
  },
});

const { BattleStream, getPlayerStreams, Teams } = require(path.resolve(
  __dirname,
  "../pokemon-showdown/sim"
));
const { RandomPlayerAI } = require(path.resolve(
  __dirname,
  "../pokemon-showdown/sim/tools/random-player-ai"
));

function ensureDir(dir) {
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
}

function writeJson(filePath, data) {
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + "\n", "utf8");
}

function parseArgs(argv) {
  const args = { outDir: "tests/showdown_compat/cases", id: null };
  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--out" && argv[i + 1]) {
      args.outDir = argv[++i];
    } else if (arg === "--id" && argv[i + 1]) {
      args.id = argv[++i];
    }
  }
  return args;
}

function extractEvents(logLines) {
  const events = { damage: [], status: [], win: null, tie: false };
  for (const line of logLines) {
    if (line.startsWith("|-damage|")) {
      const parts = line.split("|").filter(Boolean);
      // ["-damage", "p2a: Gyarados", "123/321", "..."]
      events.damage.push({
        target: parts[1] || "",
        hp: parts[2] || "",
        details: parts.slice(3),
      });
    } else if (line.startsWith("|-status|")) {
      const parts = line.split("|").filter(Boolean);
      events.status.push({
        target: parts[1] || "",
        status: parts[2] || "",
        details: parts.slice(3),
      });
    } else if (line.startsWith("|win|")) {
      const parts = line.split("|");
      events.win = parts[2] || null;
    } else if (line.startsWith("|tie|")) {
      events.tie = true;
    }
  }
  return events;
}

async function runSingleCase() {
  const seed = [1, 2, 3, 4];
  const formatid = "gen9customgame";

  const p1TeamText = [
    "Pikachu @ Oran Berry",
    "Ability: Static",
    "Level: 50",
    "EVs: 0 HP / 0 Atk / 0 Def / 0 SpA / 0 SpD / 0 Spe",
    "IVs: 31 HP / 31 Atk / 31 Def / 31 SpA / 31 SpD / 31 Spe",
    "Hardy Nature",
    "- Thunderbolt",
    "",
  ].join("\n");

  const p2TeamText = [
    "Gyarados @ Oran Berry",
    "Ability: Intimidate",
    "Level: 50",
    "EVs: 0 HP / 0 Atk / 0 Def / 0 SpA / 0 SpD / 0 Spe",
    "IVs: 31 HP / 31 Atk / 31 Def / 31 SpA / 31 SpD / 31 Spe",
    "Hardy Nature",
    "- Splash",
    "",
  ].join("\n");

  const p1Team = Teams.pack(Teams.import(p1TeamText));
  const p2Team = Teams.pack(Teams.import(p2TeamText));

  const streams = getPlayerStreams(new BattleStream());
  const p1 = new RandomPlayerAI(streams.p1, { move: 1.0, mega: 0, seed });
  const p2 = new RandomPlayerAI(streams.p2, { move: 1.0, mega: 0, seed });
  void p1.start();
  void p2.start();

  const rawChunks = [];
  const logLines = [];
  let forcedEnd = false;

  const readTask = (async () => {
    for await (const chunk of streams.omniscient) {
      rawChunks.push(chunk);
      for (const line of chunk.split("\n")) {
        if (!line) continue;
        logLines.push(line);
        if (!forcedEnd && line.startsWith("|-damage|")) {
          forcedEnd = true;
          streams.omniscient.write(`>forcewin p1`);
        }
      }
    }
  })();

  const spec = { formatid, seed };
  const p1spec = { name: "P1", team: p1Team };
  const p2spec = { name: "P2", team: p2Team };

  streams.omniscient.write(`>start ${JSON.stringify(spec)}
>player p1 ${JSON.stringify(p1spec)}
>player p2 ${JSON.stringify(p2spec)}`);

  await readTask;

  return {
    id: "pikachu_thunderbolt_vs_gyarados_splash_turn1",
    formatid,
    seed,
    p1: { name: p1spec.name, team: p1TeamText },
    p2: { name: p2spec.name, team: p2TeamText },
    rawChunks,
    log: logLines,
    events: extractEvents(logLines),
  };
}

async function main() {
  const args = parseArgs(process.argv);
  const outDir = path.resolve(process.cwd(), args.outDir);
  ensureDir(outDir);

  const testCase = await runSingleCase();
  const id = args.id || testCase.id;
  testCase.id = id;

  const outPath = path.join(outDir, `${id}.json`);
  writeJson(outPath, testCase);
  process.stdout.write(`generated: ${outPath}\n`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

