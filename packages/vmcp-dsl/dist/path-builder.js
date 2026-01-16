"use strict";
/**
 * Type-safe path builder for JSONPath expressions
 *
 * Usage:
 *   const $ = createPathBuilder<MyType>();
 *   $.data.items[0].name  // -> "$.data.items[0].name"
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.$ = void 0;
exports.createPathBuilder = createPathBuilder;
exports.getPath = getPath;
exports.path = path;
exports.index = index;
exports.wildcard = wildcard;
// Proxy handler for building JSONPath strings
const pathProxyHandler = {
    get(target, prop) {
        if (prop === 'toString' || prop === Symbol.toPrimitive) {
            return () => target.path;
        }
        if (prop === '__path__') {
            return target.path;
        }
        if (typeof prop === 'string') {
            const newPath = `${target.path}.${prop}`;
            return new Proxy({ path: newPath }, pathProxyHandler);
        }
        return undefined;
    },
};
// Symbol to extract path from proxy
const PATH_SYMBOL = Symbol('path');
/**
 * Create a type-safe path builder
 *
 * @example
 * interface Data {
 *   user: { name: string; email: string };
 *   items: { id: number; title: string }[];
 * }
 *
 * const $ = createPathBuilder<Data>();
 * const namePath = $.user.name;  // PathBuilder with path "$.user.name"
 * String(namePath)  // "$.user.name"
 */
function createPathBuilder() {
    return new Proxy({ path: '$' }, pathProxyHandler);
}
/**
 * Extract the path string from a path builder
 */
function getPath(builder) {
    // Force conversion to string
    return String(builder);
}
/**
 * Simple $ helper that returns a path string
 * Use when you don't need type safety
 */
exports.$ = {
    /**
     * Create a path expression
     * @example $._("data.items[0].name") -> "$.data.items[0].name"
     */
    _: (path) => (path ? `$.${path}` : '$'),
    /**
     * Root path
     */
    root: '$',
};
/**
 * Shorthand for creating path expressions
 * @example path("data.items") -> "$.data.items"
 */
function path(expr) {
    if (expr.startsWith('$')) {
        return expr;
    }
    return expr ? `$.${expr}` : '$';
}
/**
 * Array index access
 * @example index("items", 0) -> "$.items[0]"
 */
function index(basePath, idx) {
    const base = path(basePath);
    return `${base}[${idx}]`;
}
/**
 * Wildcard array access
 * @example wildcard("items") -> "$.items[*]"
 */
function wildcard(basePath) {
    const base = path(basePath);
    return `${base}[*]`;
}
//# sourceMappingURL=path-builder.js.map