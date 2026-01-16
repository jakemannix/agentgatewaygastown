#!/usr/bin/env node
/**
 * vmcp-compile CLI
 *
 * Compiles TypeScript vMCP definitions to JSON IR
 *
 * Usage:
 *   vmcp-compile <input.ts> [-o <output.json>]
 *   vmcp-compile --help
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { pathToFileURL } from 'node:url';

interface CliOptions {
  input: string;
  output?: string;
  validate: boolean;
  pretty: boolean;
}

function printUsage(): void {
  console.log(`
vmcp-compile - Compile vMCP TypeScript definitions to JSON IR

Usage:
  vmcp-compile <input.ts> [options]

Options:
  -o, --output <file>   Output file (default: stdout)
  --validate            Validate the registry before output
  --no-pretty           Output minified JSON
  -h, --help            Show this help message

Examples:
  vmcp-compile tools.ts -o registry.json
  vmcp-compile tools.ts --validate
  cat tools.ts | vmcp-compile -
`);
}

function parseArgs(args: string[]): CliOptions | null {
  const options: CliOptions = {
    input: '',
    validate: false,
    pretty: true,
  };

  let i = 0;
  while (i < args.length) {
    const arg = args[i];

    if (arg === '-h' || arg === '--help') {
      printUsage();
      process.exit(0);
    } else if (arg === '-o' || arg === '--output') {
      i++;
      options.output = args[i];
    } else if (arg === '--validate') {
      options.validate = true;
    } else if (arg === '--no-pretty') {
      options.pretty = false;
    } else if (!arg.startsWith('-')) {
      options.input = arg;
    } else {
      console.error(`Unknown option: ${arg}`);
      return null;
    }

    i++;
  }

  if (!options.input) {
    console.error('Error: Input file is required');
    return null;
  }

  return options;
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    printUsage();
    process.exit(1);
  }

  const options = parseArgs(args);
  if (!options) {
    process.exit(1);
  }

  try {
    // Read input file
    const inputPath = path.resolve(options.input);

    if (!fs.existsSync(inputPath)) {
      console.error(`Error: File not found: ${inputPath}`);
      process.exit(1);
    }

    // Dynamic import of the TypeScript/JavaScript file
    // The file should export a `registry` or `tools` array
    const fileUrl = pathToFileURL(inputPath).href;
    const module = await import(fileUrl);

    let registry;

    if (module.registry) {
      // Direct registry export
      registry = module.registry;
    } else if (module.tools && Array.isArray(module.tools)) {
      // Array of tools export
      registry = {
        schemaVersion: '1.0',
        tools: module.tools,
      };
    } else if (module.default) {
      // Default export
      if (Array.isArray(module.default)) {
        registry = {
          schemaVersion: '1.0',
          tools: module.default,
        };
      } else {
        registry = module.default;
      }
    } else {
      console.error('Error: Input file must export `registry`, `tools`, or a default');
      process.exit(1);
    }

    // Validate if requested
    if (options.validate) {
      const { createRegistry } = await import('../compiler.js');
      const builder = createRegistry();
      for (const tool of registry.tools || []) {
        builder.add(tool);
      }
      const result = builder.validate();

      if (!result.valid) {
        console.error('Validation errors:');
        for (const error of result.errors) {
          console.error(`  - ${error.message}`);
        }
        process.exit(1);
      }

      if (result.warnings.length > 0) {
        console.warn('Warnings:');
        for (const warning of result.warnings) {
          console.warn(`  - ${warning.message}`);
        }
      }
    }

    // Output JSON
    const json = options.pretty
      ? JSON.stringify(registry, null, 2)
      : JSON.stringify(registry);

    if (options.output) {
      fs.writeFileSync(options.output, json + '\n');
      console.error(`Written to ${options.output}`);
    } else {
      console.log(json);
    }
  } catch (error) {
    console.error('Error:', error instanceof Error ? error.message : error);
    process.exit(1);
  }
}

main();

