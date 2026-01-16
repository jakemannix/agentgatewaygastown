/**
 * Pattern builders
 */

// Stateless patterns (implemented in runtime)
export * from './pipeline.js';
export * from './scatter-gather.js';
export * from './filter.js';
export * from './schema-map.js';
export * from './map-each.js';

// Stateful patterns (IR defined, runtime not yet implemented)
export * from './stateful.js';

